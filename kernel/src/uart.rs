use core::fmt::{self, Write};

const UART_BASE: usize = 0x1000_0000;

// NS16550A UART register offsets
const RBR: usize = 0x00; // Receiver Buffer Register (read)
const THR: usize = 0x00; // Transmitter Holding Register (write)
const IER: usize = 0x01; // Interrupt Enable Register
const FCR: usize = 0x02; // FIFO Control Register (write)
const LCR: usize = 0x03; // Line Control Register
const LSR: usize = 0x05; // Line Status Register

// LSR bits
const LSR_RX_READY: u8 = 0x01; // Data ready
const LSR_TX_IDLE: u8 = 0x20;  // THR empty (Transmitter Holding Register Empty)

pub struct Console;

impl Console {
    pub const fn new() -> Self {
        Self
    }

    /// Initialize the UART for QEMU compatibility
    /// This sets up the UART with 8N1 configuration and enables FIFOs
    #[allow(dead_code)]
    pub fn init() {
        unsafe {
            let base = UART_BASE as *mut u8;
            
            // Disable all interrupts
            core::ptr::write_volatile(base.add(IER), 0x00);
            
            // Enable DLAB (Divisor Latch Access Bit) to set baud rate
            core::ptr::write_volatile(base.add(LCR), 0x80);
            
            // Set divisor to 1 (115200 baud with 1.8432 MHz clock)
            // DLL (Divisor Latch Low)
            core::ptr::write_volatile(base.add(0), 0x01);
            // DLM (Divisor Latch High)
            core::ptr::write_volatile(base.add(1), 0x00);
            
            // 8 bits, no parity, one stop bit (8N1), disable DLAB
            core::ptr::write_volatile(base.add(LCR), 0x03);
            
            // Enable FIFO, clear TX/RX queues, set 14-byte threshold
            core::ptr::write_volatile(base.add(FCR), 0xC7);
            
            // Enable receiver interrupts (optional, not strictly needed for polling)
            // core::ptr::write_volatile(base.add(IER), 0x01);
        }
    }

    #[inline(always)]
    fn lsr() -> u8 {
        unsafe { core::ptr::read_volatile((UART_BASE + LSR) as *const u8) }
    }

    #[inline(always)]
    fn wait_for_tx_ready() {
        // Wait until THR is empty (LSR bit 5)
        while (Self::lsr() & LSR_TX_IDLE) == 0 {
            core::hint::spin_loop();
        }
    }

    #[inline(always)]
    fn is_rx_ready() -> bool {
        (Self::lsr() & LSR_RX_READY) != 0
    }

    pub fn write_byte(&mut self, byte: u8) {
        Self::wait_for_tx_ready();
        unsafe {
            core::ptr::write_volatile((UART_BASE + THR) as *mut u8, byte);
        }
    }

    pub fn read_byte(&self) -> u8 {
        // Only return a byte if data is ready, otherwise return 0
        if Self::is_rx_ready() {
            unsafe { core::ptr::read_volatile((UART_BASE + RBR) as *const u8) }
        } else {
            0
        }
    }
}

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

/// Write a raw string to the UART without using `core::fmt`.
pub fn write_str(s: &str) {
    let mut console = Console::new();
    let _ = console.write_str(s);
}

/// Write a raw string followed by `\n`.
pub fn write_line(s: &str) {
    write_str(s);
    write_str("\n");
}

/// Write a raw byte slice to the UART.
pub fn write_bytes(bytes: &[u8]) {
    let mut console = Console::new();
    for &b in bytes {
        console.write_byte(b);
    }
}

/// Write an unsigned integer in decimal.
pub fn write_u64(mut n: u64) {
    let mut console = Console::new();

    if n == 0 {
        console.write_byte(b'0');
        return;
    }

    let mut buf = [0u8; 20]; // enough for u64
    let mut i = 0;

    while n > 0 && i < buf.len() {
        let digit = (n % 10) as u8;
        buf[i] = b'0' + digit;
        n /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        console.write_byte(buf[i]);
    }
}

/// Write an unsigned integer in hexadecimal.
pub fn write_hex(mut n: u64) {
    let mut console = Console::new();
    let hex_digits = b"0123456789abcdef";

    if n == 0 {
        console.write_byte(b'0');
        return;
    }

    let mut buf = [0u8; 16]; // enough for u64 hex
    let mut i = 0;

    while n > 0 && i < buf.len() {
        buf[i] = hex_digits[(n & 0xf) as usize];
        n >>= 4;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        console.write_byte(buf[i]);
    }
}

/// Write a single byte in hexadecimal (2 characters).
pub fn write_hex_byte(b: u8) {
    let mut console = Console::new();
    let hex_digits = b"0123456789abcdef";
    console.write_byte(hex_digits[(b >> 4) as usize]);
    console.write_byte(hex_digits[(b & 0xf) as usize]);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::uart::print_fmt(core::format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr $(, $($arg:tt)*)?) => ({
        $crate::uart::print_fmt(core::format_args!(concat!($fmt, "\n") $(, $($arg)*)?));
    });
}
