//! Kernel logging infrastructure
//!
//! Provides a ring buffer for kernel messages that can be:
//! - Written to by any subsystem via klog!() macro
//! - Flushed to /var/log/kernel.log by the klogd daemon
//! - Viewed via dmesg command

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::format;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::Spinlock;

/// Maximum messages in the ring buffer
const LOG_BUFFER_SIZE: usize = 128;

/// Maximum length of a single log message
const MAX_MESSAGE_LEN: usize = 256;

/// Log levels (similar to Linux kernel log levels)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    /// System is unusable
    Emergency = 0,
    /// Action must be taken immediately
    Alert = 1,
    /// Critical conditions
    Critical = 2,
    /// Error conditions
    Error = 3,
    /// Warning conditions
    Warning = 4,
    /// Normal but significant condition
    Notice = 5,
    /// Informational
    Info = 6,
    /// Debug-level messages
    Debug = 7,
    /// Trace-level messages (very verbose)
    Trace = 8,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Emergency => "EMERG",
            LogLevel::Alert => "ALERT",
            LogLevel::Critical => "CRIT",
            LogLevel::Error => "ERROR",
            LogLevel::Warning => "WARN",
            LogLevel::Notice => "NOTICE",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
    
    pub fn color(&self) -> &'static str {
        match self {
            LogLevel::Emergency | LogLevel::Alert | LogLevel::Critical => "\x1b[1;31m",
            LogLevel::Error => "\x1b[31m",
            LogLevel::Warning => "\x1b[33m",
            LogLevel::Notice => "\x1b[36m",
            LogLevel::Info => "\x1b[0m",
            LogLevel::Debug => "\x1b[90m",
            LogLevel::Trace => "\x1b[90m",
        }
    }
}

/// A single log entry
#[derive(Clone)]
pub struct LogEntry {
    /// Timestamp (ms since boot)
    pub timestamp: u64,
    /// Log level
    pub level: LogLevel,
    /// Subsystem name (e.g., "sched", "fs", "net")
    pub subsystem: String,
    /// The log message
    pub message: String,
    /// Hart that logged this
    pub hart_id: usize,
}

impl LogEntry {
    /// Format as a string for display
    pub fn format(&self) -> String {
        format!(
            "[{:>10}.{:03}] {} [{}] {}: {}",
            self.timestamp / 1000,
            self.timestamp % 1000,
            self.level.as_str(),
            self.hart_id,
            self.subsystem,
            self.message
        )
    }
    
    /// Format with colors for terminal
    pub fn format_colored(&self) -> String {
        format!(
            "\x1b[90m[{:>10}.{:03}]\x1b[0m {}{}\x1b[0m \x1b[36m[{}]\x1b[0m \x1b[33m{}:\x1b[0m {}",
            self.timestamp / 1000,
            self.timestamp % 1000,
            self.level.color(),
            self.level.as_str(),
            self.hart_id,
            self.subsystem,
            self.message
        )
    }
}

/// Ring buffer for kernel log messages
pub struct LogBuffer {
    /// Log entries
    entries: Spinlock<VecDeque<LogEntry>>,
    /// Sequence number for ordering
    sequence: AtomicUsize,
    /// Current log level filter (messages below this are suppressed)
    level_filter: AtomicUsize,
    /// Whether to also print to console
    console_enabled: AtomicBool,
    /// Whether logging is enabled
    enabled: AtomicBool,
}

impl LogBuffer {
    pub const fn new() -> Self {
        Self {
            entries: Spinlock::new(VecDeque::new()),
            sequence: AtomicUsize::new(0),
            level_filter: AtomicUsize::new(LogLevel::Info as usize),
            console_enabled: AtomicBool::new(true),
            enabled: AtomicBool::new(true),
        }
    }
    
    /// Create a new log buffer with console output disabled
    /// Useful during boot to avoid UART contention
    pub const fn new_console_disabled() -> Self {
        Self {
            entries: Spinlock::new(VecDeque::new()),
            sequence: AtomicUsize::new(0),
            level_filter: AtomicUsize::new(LogLevel::Debug as usize),
            console_enabled: AtomicBool::new(false),
            enabled: AtomicBool::new(true),
        }
    }
    
    /// Log a message
    pub fn log(&self, level: LogLevel, subsystem: &str, message: &str) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        // Check level filter
        if (level as usize) > self.level_filter.load(Ordering::Relaxed) {
            return;
        }
        
        let timestamp = crate::get_time_ms() as u64;
        let hart_id = crate::get_hart_id();
        
        // Truncate message if too long
        let message = if message.len() > MAX_MESSAGE_LEN {
            let mut s = String::from(&message[..MAX_MESSAGE_LEN - 3]);
            s.push_str("...");
            s
        } else {
            String::from(message)
        };
        
        let entry = LogEntry {
            timestamp,
            level,
            subsystem: String::from(subsystem),
            message,
            hart_id,
        };
        
        // Print to console if enabled
        if self.console_enabled.load(Ordering::Relaxed) && level <= LogLevel::Info {
            crate::uart::write_line(&entry.format_colored());
        }
        
        // Add to buffer
        let mut buffer = self.entries.lock();
        if buffer.len() >= LOG_BUFFER_SIZE {
            buffer.pop_front(); // Drop oldest
        }
        buffer.push_back(entry);
        
        self.sequence.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Drain all entries for writing to log file
    pub fn drain(&self) -> Vec<LogEntry> {
        let mut buffer = self.entries.lock();
        buffer.drain(..).collect()
    }
    
    /// Get recent entries without removing them
    pub fn recent(&self, count: usize) -> Vec<LogEntry> {
        let buffer = self.entries.lock();
        buffer.iter().rev().take(count).cloned().collect()
    }
    
    /// Get all entries without removing them
    pub fn all(&self) -> Vec<LogEntry> {
        self.entries.lock().iter().cloned().collect()
    }
    
    /// Set the log level filter
    pub fn set_level(&self, level: LogLevel) {
        self.level_filter.store(level as usize, Ordering::Release);
    }
    
    /// Enable/disable console output
    pub fn set_console(&self, enabled: bool) {
        self.console_enabled.store(enabled, Ordering::Release);
    }
    
    /// Get current entry count
    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.entries.lock().is_empty()
    }
    
    /// Get sequence number (total messages logged)
    pub fn sequence(&self) -> usize {
        self.sequence.load(Ordering::Relaxed)
    }
}

/// Global kernel log buffer
/// Note: Console output is disabled by default to avoid UART contention during boot
pub static KLOG: LogBuffer = LogBuffer::new_console_disabled();

// ═══════════════════════════════════════════════════════════════════════════════
// PUBLIC LOGGING FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Log an emergency message
pub fn klog_emergency(subsystem: &str, message: &str) {
    KLOG.log(LogLevel::Emergency, subsystem, message);
}

/// Log an alert message
pub fn klog_alert(subsystem: &str, message: &str) {
    KLOG.log(LogLevel::Alert, subsystem, message);
}

/// Log a critical message
pub fn klog_critical(subsystem: &str, message: &str) {
    KLOG.log(LogLevel::Critical, subsystem, message);
}

/// Log an error message
pub fn klog_error(subsystem: &str, message: &str) {
    KLOG.log(LogLevel::Error, subsystem, message);
}

/// Log a warning message
pub fn klog_warning(subsystem: &str, message: &str) {
    KLOG.log(LogLevel::Warning, subsystem, message);
}

/// Log a notice message
pub fn klog_notice(subsystem: &str, message: &str) {
    KLOG.log(LogLevel::Notice, subsystem, message);
}

/// Log an info message
pub fn klog_info(subsystem: &str, message: &str) {
    KLOG.log(LogLevel::Info, subsystem, message);
}

/// Log a debug message
pub fn klog_debug(subsystem: &str, message: &str) {
    KLOG.log(LogLevel::Debug, subsystem, message);
}

/// Log a trace message
pub fn klog_trace(subsystem: &str, message: &str) {
    KLOG.log(LogLevel::Trace, subsystem, message);
}

/// Set the minimum log level to display
pub fn set_log_level(level: LogLevel) {
    KLOG.set_level(level);
}

/// Enable or disable console output
pub fn set_console_output(enabled: bool) {
    KLOG.set_console(enabled);
}

