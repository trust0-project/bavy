use crate::dram::MemoryError;
use std::collections::VecDeque;
use std::sync::Mutex;

pub const UART_BASE: u64 = 0x1000_0000;
pub const UART_SIZE: u64 = 0x100;

// Registers (offset)
const RBR: u64 = 0x00; // Receiver Buffer (Read)
const THR: u64 = 0x00; // Transmitter Holding (Write)
const IER: u64 = 0x01; // Interrupt Enable
const IIR: u64 = 0x02; // Interrupt Identity (Read)
const FCR: u64 = 0x02; // FIFO Control (Write)
const LCR: u64 = 0x03; // Line Control
const MCR: u64 = 0x04; // Modem Control
const LSR: u64 = 0x05; // Line Status
const MSR: u64 = 0x06; // Modem Status
const SCR: u64 = 0x07; // Scratch

/// Internal mutable state for UART, protected by Mutex
struct UartState {
    input: VecDeque<u8>,
    output: VecDeque<u8>,
    
    // Registers
    ier: u8,
    iir: u8,
    fcr: u8,
    lcr: u8,
    mcr: u8,
    lsr: u8,
    msr: u8,
    scr: u8,
    
    // Divisor
    dll: u8,
    dlm: u8,

    interrupting: bool,
    
    /// Internal state to track if THRE interrupt is pending (waiting for IIR read or THR write).
    /// This separates the "condition" (THR empty) from the "event" (Interrupt Pending).
    thre_ip: bool,
}

pub struct Uart {
    state: Mutex<UartState>,
}

impl Uart {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(UartState {
                input: VecDeque::new(),
                output: VecDeque::new(),
                
                ier: 0x00,
                iir: 0x01, // Default: no interrupt pending
                fcr: 0x00,
                lcr: 0x00,
                mcr: 0x00,
                lsr: 0x60, // Transmitter Empty (bit 5) | Transmitter Holding Register Empty (bit 6)
                msr: 0x00,
                scr: 0x00,
                
                dll: 0x00,
                dlm: 0x00,

                interrupting: false,
                thre_ip: true, // Starts empty, so initial state could be pending if enabled
            }),
        }
    }

    fn update_interrupts(state: &mut UartState) {
        state.interrupting = false;
        state.iir = 0x01; // Default: no interrupt pending

        // Priority 1: Receiver Line Status (not implemented extensively)

        // Priority 2: Received Data Available
        if (state.lsr & 0x01) != 0 && (state.ier & 0x01) != 0 {
            state.interrupting = true;
            state.iir = 0x04; // Recieved Data Available
            return;
        }

        // Priority 3: Transmitter Holding Register Empty
        // Triggered if THR is empty AND IER bit 1 is set AND we haven't acknowledged it yet.
        if state.thre_ip && (state.ier & 0x02) != 0 {
             state.interrupting = true;
             state.iir = 0x02; // THRE
             return;
        }
        
        // Priority 4: Modem Status (not implemented)
    }

    /// Check if the UART is currently signaling an interrupt
    pub fn is_interrupting(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.interrupting
    }

    // Snapshot support methods

    /// Get input FIFO contents for snapshot
    pub fn get_input(&self) -> Vec<u8> {
        let state = self.state.lock().unwrap();
        state.input.iter().copied().collect()
    }

    /// Get output FIFO contents for snapshot
    pub fn get_output(&self) -> Vec<u8> {
        let state = self.state.lock().unwrap();
        state.output.iter().copied().collect()
    }

    /// Get the number of bytes pending in the output FIFO
    pub fn output_len(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.output.len()
    }

    /// Get all register values for snapshot
    pub fn get_registers(&self) -> (u8, u8, u8, u8, u8, u8, u8, u8, u8, u8) {
        let state = self.state.lock().unwrap();
        (state.ier, state.iir, state.fcr, state.lcr, state.mcr, 
         state.lsr, state.msr, state.scr, state.dll, state.dlm)
    }

    /// Restore input FIFO from snapshot
    pub fn set_input(&self, values: &[u8]) {
        let mut state = self.state.lock().unwrap();
        state.input.clear();
        for &v in values {
            state.input.push_back(v);
        }
        // Update LSR data ready bit
        if !state.input.is_empty() {
            state.lsr |= 0x01;
        } else {
            state.lsr &= !0x01;
        }
    }

    /// Restore output FIFO from snapshot
    pub fn set_output(&self, values: &[u8]) {
        let mut state = self.state.lock().unwrap();
        state.output.clear();
        for &v in values {
            state.output.push_back(v);
        }
    }

    /// Restore register values from snapshot
    pub fn set_registers(&self, ier: u8, iir: u8, fcr: u8, lcr: u8, mcr: u8,
                         lsr: u8, msr: u8, scr: u8, dll: u8, dlm: u8) {
        let mut state = self.state.lock().unwrap();
        state.ier = ier;
        state.iir = iir;
        state.fcr = fcr;
        state.lcr = lcr;
        state.mcr = mcr;
        state.lsr = lsr;
        state.msr = msr;
        state.scr = scr;
        state.dll = dll;
        state.dlm = dlm;
        Self::update_interrupts(&mut state);
    }

    pub fn load(&self, offset: u64, size: u64) -> Result<u64, MemoryError> {
        let mut state = self.state.lock().unwrap();
        if size != 1 {
            // Some OS might try 4-byte reads, technically not allowed by spec but we can be lenient or strict.
            // Spec says 8-bit width. Let's return 0 for now if not byte access.
            return Ok(0);
        }

        let val = match offset {
            RBR => {
                if (state.lcr & 0x80) != 0 {
                    state.dll
                } else {
                    // RBR: Read from input FIFO
                    let byte = state.input.pop_front().unwrap_or(0);
                    // Update LSR: if more data, set bit 0, else clear it
                    if state.input.is_empty() {
                        state.lsr &= !0x01;
                    } else {
                        state.lsr |= 0x01;
                    }
                    Self::update_interrupts(&mut state);
                    byte
                }
            }
            IER => {
                if (state.lcr & 0x80) != 0 {
                    state.dlm
                } else {
                    state.ier
                }
            }
            IIR => {
                let val = state.iir;
                // Reading IIR clears THRE interrupt if it is the indicated interrupt
                if (val & 0x0F) == 0x02 {
                    state.thre_ip = false;
                    Self::update_interrupts(&mut state);
                    log::trace!("[UART] IIR read cleared THRE ip");
                } else {
                    log::trace!("[UART] IIR read val={:x} (thre_ip={})", val, state.thre_ip);
                }
                val
            }
            LCR => state.lcr,
            MCR => state.mcr,
            LSR => state.lsr,
            MSR => state.msr,
            SCR => state.scr,
            _ => 0,
        };

        Ok(val as u64)
    }

    pub fn store(&self, offset: u64, size: u64, value: u64) -> Result<(), MemoryError> {
        let mut state = self.state.lock().unwrap();
        if size != 1 {
            return Ok(());
        }

        let val = (value & 0xff) as u8;

        match offset {
            THR => {
                if (state.lcr & 0x80) != 0 {
                    state.dll = val;
                } else {
                    // THR: Write to output
                    log::trace!(
                        "[UART] TX '{}' (0x{:02x})",
                        if val.is_ascii_graphic() {
                            val as char
                        } else {
                            '.'
                        },
                        val
                    );
                    state.output.push_back(val);
                    // We instantly "transmit", so THRE (bit 5) is always set.
                    // Writing to THR clears the THRE interrupt (if pending),
                    // but since it becomes empty immediately, we set thre_ip to true again?
                    // In real HW, it goes Not Empty -> Empty.
                    // So we should clear it, then re-assert it.
                    // For edge-triggered emulation, simply re-asserting is correct because we transitioned.
                    state.lsr |= 0x20; 
                    state.thre_ip = true; 
                    Self::update_interrupts(&mut state);
                }
            }
            IER => {
                if (state.lcr & 0x80) != 0 {
                    state.dlm = val;
                } else {
                    state.ier = val;
                    Self::update_interrupts(&mut state);
                }
            }
            FCR => {
                state.fcr = val;
                if (state.fcr & 0x02) != 0 {
                    state.input.clear();
                    state.lsr &= !0x01;
                }
                if (state.fcr & 0x04) != 0 {
                    state.output.clear();
                    state.lsr |= 0x60; // Empty
                }
                Self::update_interrupts(&mut state);
            }
            LCR => {
                state.lcr = val;
            }
            MCR => {
                state.mcr = val;
            }
            LSR => {
                // Usually read-only, but factory test mode might write. Ignore.
            }
            MSR => {
                // Read-only.
            }
            SCR => {
                state.scr = val;
            }
            _ => {}
        }
        Ok(())
    }

    // Interface for the Host
    pub fn push_input(&self, byte: u8) {
        let mut state = self.state.lock().unwrap();
        state.input.push_back(byte);
        state.lsr |= 0x01; // Data Ready
        Self::update_interrupts(&mut state);
    }

    pub fn pop_output(&self) -> Option<u8> {
        let mut state = self.state.lock().unwrap();
        state.output.pop_front()
    }

    /// Check if UART has pending output.
    pub fn has_output(&self) -> bool {
        !self.state.lock().unwrap().output.is_empty()
    }
    
    /// Drain all pending output bytes in a single lock acquisition.
    /// More efficient than calling pop_output() in a loop.
    pub fn drain_output(&self) -> Vec<u8> {
        let mut state = self.state.lock().unwrap();
        state.output.drain(..).collect()
    }

    /// Clear interrupt flag.
    pub fn clear_interrupt(&self) {
        self.state.lock().unwrap().interrupting = false;
    }

    /// Push a byte directly to the output queue.
    /// This is used by the VM itself to emit messages (banners, status, etc.)
    /// that should appear in the same output stream as guest UART output.
    pub fn push_output(&self, byte: u8) {
        let mut state = self.state.lock().unwrap();
        state.output.push_back(byte);
    }

    /// Push a string directly to the output queue.
    /// Convenience method for emitting VM messages.
    pub fn push_output_str(&self, s: &str) {
        let mut state = self.state.lock().unwrap();
        for b in s.bytes() {
            state.output.push_back(b);
        }
    }
}


