//! Emulated D1 Audio Codec
//!
//! Emulates an Allwinner D1-style audio codec (I2S/DAC) for the VM.
//! Audio samples written by the kernel are buffered and can be extracted
//! by the host for playback via WebAudio (browser) or native audio APIs.
//!
//! This allows the kernel to use a D1-compatible audio driver on both
//! real hardware and the emulator.

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

// Audio codec base address (D1 audio codec region)
pub const D1_AUDIO_BASE: u64 = 0x0203_0000;
pub const D1_AUDIO_SIZE: u64 = 0x1000;

// Register offsets
const CODEC_CTL: u64 = 0x00;      // Control: enable, sample rate
const CODEC_STS: u64 = 0x04;      // Status: buffer level, underrun flag
const CODEC_DATA: u64 = 0x08;     // Sample FIFO write (32-bit: left + right 16-bit samples)
const CODEC_BUF_LEVEL: u64 = 0x0C; // Current buffer fill level (read-only)
const CODEC_SAMPLE_RATE: u64 = 0x10; // Sample rate register

// Control register bits
const CTL_ENABLE: u32 = 1 << 0;
const CTL_RESET: u32 = 1 << 1;

// Status register bits
const STS_BUFFER_FULL: u32 = 1 << 0;
const STS_BUFFER_EMPTY: u32 = 1 << 1;
const STS_UNDERRUN: u32 = 1 << 2;

// Default audio parameters
const DEFAULT_SAMPLE_RATE: u32 = 48000;
const BUFFER_CAPACITY: usize = 16384; // ~16KB buffer (~170ms at 48kHz stereo)

/// Emulated D1 Audio Codec
pub struct D1AudioEmulated {
    // Audio sample buffer (interleaved 16-bit stereo samples as u32)
    buffer: Arc<RwLock<VecDeque<u32>>>,
    
    // Control register
    ctl: u32,
    
    // Sample rate in Hz
    sample_rate: u32,
    
    // Underrun flag (set when buffer empties during playback)
    underrun: bool,
    
    // Device enabled state
    enabled: bool,
}

impl D1AudioEmulated {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(BUFFER_CAPACITY))),
            ctl: 0,
            sample_rate: DEFAULT_SAMPLE_RATE,
            underrun: false,
            enabled: false,
        }
    }

    /// Check if audio is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get current sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get current buffer level (number of samples)
    pub fn buffer_level(&self) -> usize {
        self.buffer.read().unwrap().len()
    }

    /// Extract audio samples for playback.
    /// Returns samples as (left, right) i16 pairs converted to f32 [-1.0, 1.0].
    /// The host should call this periodically to drain the buffer.
    pub fn extract_samples(&self, max_samples: usize) -> Vec<f32> {
        let mut buf = self.buffer.write().unwrap();
        let count = buf.len().min(max_samples);
        
        let mut samples = Vec::with_capacity(count * 2);
        for _ in 0..count {
            if let Some(sample) = buf.pop_front() {
                // Unpack interleaved stereo: low 16 bits = left, high 16 bits = right
                let left = (sample & 0xFFFF) as i16;
                let right = ((sample >> 16) & 0xFFFF) as i16;
                
                // Convert to f32 [-1.0, 1.0]
                samples.push(left as f32 / 32768.0);
                samples.push(right as f32 / 32768.0);
            }
        }
        
        samples
    }

    /// Check if buffer has samples available
    pub fn has_samples(&self) -> bool {
        !self.buffer.read().unwrap().is_empty()
    }

    /// Reset the audio device
    fn reset(&mut self) {
        self.buffer.write().unwrap().clear();
        self.ctl = 0;
        self.underrun = false;
        self.enabled = false;
    }
}

// MMIO Access Methods (for bus integration)
impl D1AudioEmulated {
    pub fn mmio_read32(&self, addr: u64) -> u32 {
        let offset = addr - D1_AUDIO_BASE;
        
        match offset {
            CODEC_CTL => self.ctl,
            
            CODEC_STS => {
                let buf = self.buffer.read().unwrap();
                let mut status = 0u32;
                
                if buf.len() >= BUFFER_CAPACITY {
                    status |= STS_BUFFER_FULL;
                }
                if buf.is_empty() {
                    status |= STS_BUFFER_EMPTY;
                }
                if self.underrun {
                    status |= STS_UNDERRUN;
                }
                
                status
            }
            
            CODEC_BUF_LEVEL => self.buffer.read().unwrap().len() as u32,
            
            CODEC_SAMPLE_RATE => self.sample_rate,
            
            _ => 0,
        }
    }

    pub fn mmio_write32(&mut self, addr: u64, value: u32) {
        let offset = addr - D1_AUDIO_BASE;
        
        match offset {
            CODEC_CTL => {
                // Handle reset bit
                if (value & CTL_RESET) != 0 {
                    self.reset();
                    return;
                }
                
                self.ctl = value;
                self.enabled = (value & CTL_ENABLE) != 0;
            }
            
            CODEC_DATA => {
                if self.enabled {
                    let mut buf = self.buffer.write().unwrap();
                    
                    // Drop oldest samples if buffer is full (prevents blocking)
                    while buf.len() >= BUFFER_CAPACITY {
                        buf.pop_front();
                    }
                    
                    buf.push_back(value);
                }
            }
            
            CODEC_SAMPLE_RATE => {
                // Common sample rates: 44100, 48000, 22050, 11025
                self.sample_rate = value;
            }
            
            _ => {}
        }
    }

    pub fn mmio_read8(&self, addr: u64) -> u8 {
        let word = self.mmio_read32(addr & !3);
        let shift = (addr & 3) * 8;
        (word >> shift) as u8
    }

    #[allow(unused_variables)]
    pub fn mmio_write8(&mut self, addr: u64, value: u8) {
        // Byte writes not commonly used for audio
    }
}

impl Default for D1AudioEmulated {
    fn default() -> Self {
        Self::new()
    }
}
