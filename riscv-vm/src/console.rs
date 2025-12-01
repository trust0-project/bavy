//! Non-blocking console I/O for native builds.

#![cfg(not(target_arch = "wasm32"))]

use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};

/// Non-blocking console input handler.
/// 
/// Spawns a background thread that reads from stdin
/// and makes bytes available via `try_read()`.
pub struct Console {
    rx: Receiver<u8>,
    _handle: Option<JoinHandle<()>>,
}

impl Console {
    /// Create a new console with a background input thread.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        let handle = thread::Builder::new()
            .name("console-input".to_string())
            .spawn(move || {
                let stdin = io::stdin();
                let mut buffer = [0u8; 1];
                
                // Set terminal to raw mode
                #[cfg(unix)]
                let _raw_guard = RawModeGuard::new();
                
                loop {
                    match stdin.lock().read(&mut buffer) {
                        Ok(1) => {
                            if tx.send(buffer[0]).is_err() {
                                // Receiver dropped, exit
                                break;
                            }
                        }
                        Ok(0) => {
                            // EOF
                            break;
                        }
                        Err(e) => {
                            if e.kind() != io::ErrorKind::Interrupted {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            })
            .expect("Failed to spawn console thread");

        Self { rx, _handle: Some(handle) }
    }

    /// Try to read a byte from stdin (non-blocking).
    /// 
    /// Returns `Some(byte)` if input is available, `None` otherwise.
    pub fn try_read(&self) -> Option<u8> {
        match self.rx.try_recv() {
            Ok(byte) => Some(byte),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }

    /// Read all available input bytes.
    pub fn read_available(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        while let Some(byte) = self.try_read() {
            bytes.push(byte);
        }
        bytes
    }

    /// Alias for try_read() for backwards compatibility.
    pub fn poll(&self) -> Option<u8> {
        self.try_read()
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        // Thread will exit when channel is dropped
        // We don't explicitly join because stdin.read() blocks
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for Unix raw terminal mode.
#[cfg(unix)]
struct RawModeGuard {
    original: libc::termios,
}

#[cfg(unix)]
impl RawModeGuard {
    fn new() -> Self {
        use std::os::unix::io::AsRawFd;
        use std::mem::MaybeUninit;
        
        let fd = io::stdin().as_raw_fd();
        let mut original = MaybeUninit::<libc::termios>::uninit();
        
        unsafe {
            libc::tcgetattr(fd, original.as_mut_ptr());
            let original = original.assume_init();
            
            let mut raw = original;
            // Disable canonical mode and echo
            raw.c_lflag &= !(libc::ICANON | libc::ECHO);
            // Read returns after 1 byte
            raw.c_cc[libc::VMIN] = 1;
            raw.c_cc[libc::VTIME] = 0;
            
            libc::tcsetattr(fd, libc::TCSANOW, &raw);
            
            Self { original }
        }
    }
}

#[cfg(unix)]
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        use std::os::unix::io::AsRawFd;
        let fd = io::stdin().as_raw_fd();
        unsafe {
            libc::tcsetattr(fd, libc::TCSANOW, &self.original);
        }
        // Flush any pending output
        let _ = io::stdout().flush();
    }
}

// Windows stub
#[cfg(windows)]
struct RawModeGuard;

#[cfg(windows)]
impl RawModeGuard {
    fn new() -> Self {
        // TODO: Implement Windows console mode using SetConsoleMode
        Self
    }
}
