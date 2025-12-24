//! Emulated D1 GT911 Touchscreen Controller
//!
//! Emulates the Goodix GT911 I2C capacitive touchscreen controller.
//! On browser (WASM), receives mouse/touch events from JavaScript.
//!
//! This allows the kernel to use the same GT911 driver on both
//! real hardware (Lichee RV 86) and the emulator.
//!
//! # GT911 I2C Protocol
//! - Slave address: 0x14 (7-bit) or 0x28/0x29 (8-bit R/W)
//! - 16-bit register addressing
//! - Touch status at 0x814E
//! - Touch data at 0x814F-0x8157 (for first point)

use std::sync::{Arc, RwLock};

// GT911 I2C address (7-bit)
pub const GT911_I2C_ADDR: u8 = 0x14;

// D1 I2C Controller base (TWI2 on Lichee RV 86)
pub const D1_I2C2_BASE: u64 = 0x0250_2000;
pub const D1_I2C2_SIZE: u64 = 0x400;

// GT911 Register addresses
const GT911_PRODUCT_ID: u16 = 0x8140;      // "911\0" product ID
const GT911_FIRMWARE_VER: u16 = 0x8144;    // Firmware version
const GT911_X_RES: u16 = 0x8146;           // X resolution (2 bytes, little-endian)
const GT911_Y_RES: u16 = 0x8148;           // Y resolution (2 bytes, little-endian)
const GT911_TOUCH_STATUS: u16 = 0x814E;    // Touch status register
const GT911_TOUCH_DATA: u16 = 0x814F;      // First touch point data (8 bytes per point)
const GT911_COMMAND: u16 = 0x8040;         // Command register

// Touch status bits
const TOUCH_STATUS_BUFFER_READY: u8 = 0x80;  // Data ready to read
const TOUCH_STATUS_LARGE_DETECT: u8 = 0x40; // Large area detected
const TOUCH_STATUS_POINT_MASK: u8 = 0x0F;   // Number of touch points (0-5)

// Display parameters (matching D1 Display)
const DISPLAY_WIDTH: u16 = 1024;
const DISPLAY_HEIGHT: u16 = 768;

/// Single touch point data
#[derive(Clone, Copy, Debug, Default)]
pub struct TouchPoint {
    pub id: u8,
    pub x: u16,
    pub y: u16,
    pub size: u16,
}

/// Queued touch event (to prevent race conditions)
#[derive(Clone, Copy, Debug)]
pub struct QueuedTouchEvent {
    pub x: u16,
    pub y: u16,
    pub pressed: bool,
}

/// Queued keyboard event
#[derive(Clone, Copy, Debug)]
pub struct QueuedKeyEvent {
    pub key_code: u16,
    pub pressed: bool,
}

/// Emulated GT911 Touchscreen Controller
pub struct D1TouchEmulated {
    // Current touch state
    touch_points: Vec<TouchPoint>,
    num_points: u8,
    
    // Status flags
    buffer_ready: bool,
    pending_int: bool,
    
    // Event queue to handle rapid press/release before kernel polls
    event_queue: Vec<QueuedTouchEvent>,
    
    // Keyboard event queue (for evdev key codes)
    key_event_queue: Vec<QueuedKeyEvent>,
    
    // Character queue (for actual typed characters - respects keyboard layout)
    char_queue: Vec<u8>,
    
    // Configuration
    x_resolution: u16,
    y_resolution: u16,
    
    // I2C transaction state
    reg_addr: u16,
    addr_phase: u8,  // 0 = none, 1 = high byte received, 2 = low byte received
}

impl D1TouchEmulated {
    pub fn new() -> Self {
        Self {
            touch_points: Vec::with_capacity(5),
            num_points: 0,
            buffer_ready: false,
            pending_int: false,
            event_queue: Vec::with_capacity(16),
            key_event_queue: Vec::with_capacity(32),
            char_queue: Vec::with_capacity(64),
            x_resolution: DISPLAY_WIDTH,
            y_resolution: DISPLAY_HEIGHT,
            reg_addr: 0,
            addr_phase: 0,
        }
    }

    /// Push a touch event (called from host/JS)
    /// Events are queued and processed when kernel reads from MMIO to prevent race conditions
    pub fn push_touch(&mut self, x: u16, y: u16, pressed: bool) {

        // Queue the event for later processing
        self.event_queue.push(QueuedTouchEvent {
            x: x.min(self.x_resolution - 1),
            y: y.min(self.y_resolution - 1),
            pressed,
        });
        
        // If no current event is being served, apply the first queued event
        if !self.buffer_ready && !self.event_queue.is_empty() {
            self.apply_next_queued_event();
        }
        
    }
    
    /// Push a keyboard event (called from host/JS)
    pub fn push_key(&mut self, key_code: u16, pressed: bool) {
        self.key_event_queue.push(QueuedKeyEvent {
            key_code,
            pressed,
        });
    }
    
    /// Push a character (ASCII) from the browser - respects keyboard layout
    /// This is the preferred way to send typed characters (e.g., '/' from Shift+7)
    pub fn push_char(&mut self, ch: u8) {
        self.char_queue.push(ch);
    }
    
    /// Apply the next queued event to current state
    fn apply_next_queued_event(&mut self) {
        if let Some(event) = self.event_queue.first().cloned() {
            // Remove from queue
            self.event_queue.remove(0);
            
            if event.pressed {
                // Touch down/move
                let point = TouchPoint {
                    id: 0,
                    x: event.x,
                    y: event.y,
                    size: 50,
                };
                
                if self.touch_points.is_empty() {
                    self.touch_points.push(point);
                } else {
                    self.touch_points[0] = point;
                }
                self.num_points = 1;
            } else {
                // Touch release
                self.touch_points.clear();
                self.num_points = 0;
            }
            
            self.buffer_ready = true;
            self.pending_int = true;
        }
    }

    /// Check if interrupt is pending
    pub fn is_int_pending(&self) -> bool {
        self.pending_int
    }

    /// Clear interrupt flag (after host reads status)
    pub fn clear_int(&mut self) {
        self.pending_int = false;
    }

    /// Read from GT911 register space
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            // Product ID: "911\0"
            0x8140 => b'9',
            0x8141 => b'1',
            0x8142 => b'1',
            0x8143 => 0,
            
            // Firmware version
            0x8144 => 0x10,
            0x8145 => 0x41,
            
            // X resolution (little-endian)
            0x8146 => (self.x_resolution & 0xFF) as u8,
            0x8147 => (self.x_resolution >> 8) as u8,
            
            // Y resolution (little-endian)
            0x8148 => (self.y_resolution & 0xFF) as u8,
            0x8149 => (self.y_resolution >> 8) as u8,
            
            // Touch status
            0x814E => {
                let mut status = self.num_points & TOUCH_STATUS_POINT_MASK;
                if self.buffer_ready {
                    status |= TOUCH_STATUS_BUFFER_READY;
                }
                status
            }
            
            // Touch point data (8 bytes per point)
            // Format: track_id(1), x_low(1), x_high(1), y_low(1), y_high(1), size_low(1), size_high(1), reserved(1)
            0x814F..=0x8186 => {
                let point_offset = (addr - GT911_TOUCH_DATA) as usize;
                let point_idx = point_offset / 8;
                let byte_idx = point_offset % 8;
                
                if point_idx < self.touch_points.len() {
                    let point = &self.touch_points[point_idx];
                    match byte_idx {
                        0 => point.id,
                        1 => (point.x & 0xFF) as u8,
                        2 => (point.x >> 8) as u8,
                        3 => (point.y & 0xFF) as u8,
                        4 => (point.y >> 8) as u8,
                        5 => (point.size & 0xFF) as u8,
                        6 => (point.size >> 8) as u8,
                        7 => 0, // Reserved
                        _ => 0,
                    }
                } else {
                    0
                }
            }
            
            _ => 0,
        }
    }

    /// Write to GT911 register space
    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            // Command register
            0x8040 => {
                match value {
                    0 => {
                        // Read coordinate status - clear buffer ready
                        self.buffer_ready = false;
                    }
                    _ => {}
                }
            }
            
            // Touch status - write 0 to clear
            0x814E => {
                if value == 0 {
                    self.buffer_ready = false;
                }
            }
            
            _ => {}
        }
    }
}

// MMIO Access Methods (for I2C controller emulation)
impl D1TouchEmulated {
    /// Read from I2C controller MMIO (simplified)
    /// In a real D1, this would be a full TWI controller.
    /// We simplify by exposing GT911 registers directly via MMIO.
    pub fn mmio_read32(&mut self, addr: u64) -> u32 {
        let offset = addr - D1_I2C2_BASE;
        
        match offset {
            // INT status (custom register for emulator)
            0x100 => {
                let result = if self.pending_int { 1 } else { 0 };
                result
            }
            
            // Touch status register (direct access)
            0x104 => {
                let mut status = self.num_points as u32;
                if self.buffer_ready {
                    status |= 0x80;
                }
                status
            }
            
            // Touch X coordinate
            0x108 => {
                if !self.touch_points.is_empty() {
                    self.touch_points[0].x as u32
                } else {
                    0
                }
            }
            
            // Touch Y coordinate
            0x10C => {
                if !self.touch_points.is_empty() {
                    self.touch_points[0].y as u32
                } else {
                    0
                }
            }
            
            // Touch count
            0x110 => self.num_points as u32,
            
            // X resolution
            0x114 => self.x_resolution as u32,
            
            // Y resolution
            0x118 => self.y_resolution as u32,
            
            // Keyboard event count
            0x11C => self.key_event_queue.len() as u32,
            
            // Keyboard key code (peek front of queue)
            0x120 => {
                if let Some(event) = self.key_event_queue.first() {
                    event.key_code as u32
                } else {
                    0
                }
            }
            
            // Keyboard key state (1 = pressed, 0 = released)
            0x124 => {
                if let Some(event) = self.key_event_queue.first() {
                    if event.pressed { 1 } else { 0 }
                } else {
                    0
                }
            }
            
            // Character queue count (for typed characters respecting keyboard layout)
            0x128 => {
                let count = self.char_queue.len() as u32;
                count
            }
            
            // Character code (peek front of queue)
            0x12C => {
                if let Some(&ch) = self.char_queue.first() {
                    ch as u32
                } else {
                    0
                }
            }
            
            _ => 0,
        }
    }

    pub fn mmio_write32(&mut self, addr: u64, value: u32) {
        let offset = addr - D1_I2C2_BASE;
        
        match offset {
            // Clear INT status - but don't clobber pending_int if we just served a new event
            0x100 => {
                if value == 0 {
                    // Only clear pending_int if there's no new event waiting
                    // (buffer_ready would be true if apply_next_queued_event() just served one)
                    if !self.buffer_ready {
                        self.pending_int = false;
                    }
                }
            }
            
            // Clear touch status (acknowledge read)
            0x104 => {
                if value == 0 {
                    self.buffer_ready = false;
                    // When kernel acknowledges, serve the next queued event if any
                    if !self.event_queue.is_empty() {
                        self.apply_next_queued_event();
                    }
                }
            }
            
            // Acknowledge keyboard event (consume from queue)
            0x11C => {
                if value == 0 && !self.key_event_queue.is_empty() {
                    self.key_event_queue.remove(0);
                }
            }
            
            // Acknowledge character (consume from queue)
            0x128 => {
                if value == 0 && !self.char_queue.is_empty() {
                    self.char_queue.remove(0);
                }
            }
            
            _ => {}
        }
    }

    pub fn mmio_read8(&self, _addr: u64) -> u8 {
        0
    }

    pub fn mmio_write8(&mut self, _addr: u64, _value: u8) {
        // Not used
    }
}

impl Default for D1TouchEmulated {
    fn default() -> Self {
        Self::new()
    }
}
