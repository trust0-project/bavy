//! Inter-Process Communication (IPC) Module
//!
//! Provides message-passing primitives for task communication:
//! - Channels: Unidirectional, bounded message queues
//! - Pipes: Byte-stream communication (like Unix pipes)
//!
//! Tasks can block waiting for data, enabling efficient IPC without polling.

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::Spinlock;
use crate::task::Pid;

/// Channel identifier
pub type ChannelId = u32;

/// Maximum message size for channels (4KB)
pub const MAX_MESSAGE_SIZE: usize = 4096;

/// Maximum number of messages in a channel buffer
pub const DEFAULT_CHANNEL_CAPACITY: usize = 16;

/// Maximum pipe buffer size (8KB)
pub const PIPE_BUFFER_SIZE: usize = 8192;

// ═══════════════════════════════════════════════════════════════════════════════
// MESSAGE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// A message that can be sent through a channel
#[derive(Clone)]
pub struct Message {
    /// Sender's PID
    pub sender: Pid,
    /// Message payload (bytes)
    pub data: Vec<u8>,
    /// Message type/tag for filtering
    pub msg_type: u32,
}

impl Message {
    /// Create a new message from bytes
    pub fn new(sender: Pid, data: Vec<u8>, msg_type: u32) -> Self {
        Self { sender, data, msg_type }
    }
    
    /// Create a message from a string
    pub fn from_str(sender: Pid, s: &str, msg_type: u32) -> Self {
        Self {
            sender,
            data: s.as_bytes().to_vec(),
            msg_type,
        }
    }
    
    /// Get message data as string (lossy)
    pub fn as_str(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CHANNEL IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════════

/// A bounded, MPSC (multi-producer, single-consumer) channel
pub struct Channel {
    /// Unique channel ID
    pub id: ChannelId,
    /// Human-readable name
    pub name: String,
    /// Message queue
    buffer: Spinlock<VecDeque<Message>>,
    /// Maximum number of messages
    capacity: usize,
    /// Number of senders (for detecting orphaned channels)
    sender_count: AtomicUsize,
    /// Whether the receiver is still active
    receiver_active: AtomicBool,
    /// PIDs waiting to receive
    waiters: Spinlock<VecDeque<Pid>>,
    /// Channel is closed
    closed: AtomicBool,
}

impl Channel {
    /// Create a new channel with default capacity
    pub fn new(id: ChannelId, name: &str) -> Self {
        Self::with_capacity(id, name, DEFAULT_CHANNEL_CAPACITY)
    }
    
    /// Create a new channel with specified capacity
    pub fn with_capacity(id: ChannelId, name: &str, capacity: usize) -> Self {
        Self {
            id,
            name: String::from(name),
            buffer: Spinlock::new(VecDeque::with_capacity(capacity)),
            capacity,
            sender_count: AtomicUsize::new(1),
            receiver_active: AtomicBool::new(true),
            waiters: Spinlock::new(VecDeque::new()),
            closed: AtomicBool::new(false),
        }
    }
    
    /// Send a message to the channel (non-blocking)
    /// Returns Err if channel is full or closed
    pub fn send(&self, msg: Message) -> Result<(), &'static str> {
        if self.closed.load(Ordering::Acquire) {
            return Err("Channel closed");
        }
        
        if !self.receiver_active.load(Ordering::Acquire) {
            return Err("No receiver");
        }
        
        let mut buffer = self.buffer.lock();
        if buffer.len() >= self.capacity {
            return Err("Channel full");
        }
        
        buffer.push_back(msg);
        
        // Wake up first waiter if any
        let waiter = self.waiters.lock().pop_front();
        if let Some(pid) = waiter {
            // Signal the waiting task (would integrate with scheduler)
            crate::klog::klog_trace("ipc", &alloc::format!("Waking task {} on channel {}", pid, self.id));
        }
        
        Ok(())
    }
    
    /// Try to receive a message (non-blocking)
    /// Returns None if no message available
    pub fn try_recv(&self) -> Option<Message> {
        if self.closed.load(Ordering::Acquire) {
            return None;
        }
        
        self.buffer.lock().pop_front()
    }
    
    /// Register a task as waiting for a message
    pub fn register_waiter(&self, pid: Pid) {
        self.waiters.lock().push_back(pid);
    }
    
    /// Check if channel has messages
    pub fn has_messages(&self) -> bool {
        !self.buffer.lock().is_empty()
    }
    
    /// Get number of pending messages
    pub fn pending_count(&self) -> usize {
        self.buffer.lock().len()
    }
    
    /// Close the channel
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        // Wake all waiters
        let mut waiters = self.waiters.lock();
        while let Some(pid) = waiters.pop_front() {
            crate::klog::klog_trace("ipc", &alloc::format!("Waking blocked task {} (channel closed)", pid));
        }
    }
    
    /// Check if channel is closed
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
    
    /// Increment sender count (when cloning sender handle)
    pub fn add_sender(&self) {
        self.sender_count.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Decrement sender count (when dropping sender handle)
    pub fn remove_sender(&self) {
        let prev = self.sender_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            // Last sender dropped - close channel
            self.close();
        }
    }
    
    /// Mark receiver as dropped
    pub fn drop_receiver(&self) {
        self.receiver_active.store(false, Ordering::Release);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PIPE IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Pipe identifier
pub type PipeId = u32;

/// A unidirectional byte-stream pipe (like Unix pipes)
pub struct Pipe {
    /// Unique pipe ID
    pub id: PipeId,
    /// Circular buffer for data
    buffer: Spinlock<PipeBuffer>,
    /// Reader is still active
    reader_active: AtomicBool,
    /// Writer is still active
    writer_active: AtomicBool,
    /// PIDs waiting to read
    read_waiters: Spinlock<VecDeque<Pid>>,
    /// PIDs waiting to write (when buffer full)
    write_waiters: Spinlock<VecDeque<Pid>>,
}

/// Internal pipe buffer (circular)
struct PipeBuffer {
    data: [u8; PIPE_BUFFER_SIZE],
    read_pos: usize,
    write_pos: usize,
    count: usize,
}

impl PipeBuffer {
    const fn new() -> Self {
        Self {
            data: [0u8; PIPE_BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
            count: 0,
        }
    }
    
    fn available_read(&self) -> usize {
        self.count
    }
    
    fn available_write(&self) -> usize {
        PIPE_BUFFER_SIZE - self.count
    }
    
    fn write(&mut self, data: &[u8]) -> usize {
        let to_write = data.len().min(self.available_write());
        for &byte in &data[..to_write] {
            self.data[self.write_pos] = byte;
            self.write_pos = (self.write_pos + 1) % PIPE_BUFFER_SIZE;
            self.count += 1;
        }
        to_write
    }
    
    fn read(&mut self, buf: &mut [u8]) -> usize {
        let to_read = buf.len().min(self.available_read());
        for byte in buf[..to_read].iter_mut() {
            *byte = self.data[self.read_pos];
            self.read_pos = (self.read_pos + 1) % PIPE_BUFFER_SIZE;
            self.count -= 1;
        }
        to_read
    }
}

impl Pipe {
    /// Create a new pipe
    pub fn new(id: PipeId) -> Self {
        Self {
            id,
            buffer: Spinlock::new(PipeBuffer::new()),
            reader_active: AtomicBool::new(true),
            writer_active: AtomicBool::new(true),
            read_waiters: Spinlock::new(VecDeque::new()),
            write_waiters: Spinlock::new(VecDeque::new()),
        }
    }
    
    /// Write data to the pipe (non-blocking)
    /// Returns number of bytes written, or Err if pipe is broken
    pub fn write(&self, data: &[u8]) -> Result<usize, &'static str> {
        if !self.reader_active.load(Ordering::Acquire) {
            return Err("Broken pipe (no reader)");
        }
        
        let written = self.buffer.lock().write(data);
        
        // Wake up readers
        if written > 0 {
            if let Some(pid) = self.read_waiters.lock().pop_front() {
                crate::klog::klog_trace("ipc", &alloc::format!("Waking reader {} on pipe {}", pid, self.id));
            }
        }
        
        Ok(written)
    }
    
    /// Read data from the pipe (non-blocking)
    /// Returns number of bytes read, or Err if pipe is closed
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let read = self.buffer.lock().read(buf);
        
        // Wake up writers
        if read > 0 {
            if let Some(pid) = self.write_waiters.lock().pop_front() {
                crate::klog::klog_trace("ipc", &alloc::format!("Waking writer {} on pipe {}", pid, self.id));
            }
        }
        
        // If no data and writer is gone, return EOF
        if read == 0 && !self.writer_active.load(Ordering::Acquire) {
            return Err("EOF");
        }
        
        Ok(read)
    }
    
    /// Register a task as waiting to read
    pub fn register_read_waiter(&self, pid: Pid) {
        self.read_waiters.lock().push_back(pid);
    }
    
    /// Register a task as waiting to write
    pub fn register_write_waiter(&self, pid: Pid) {
        self.write_waiters.lock().push_back(pid);
    }
    
    /// Check if data is available to read
    pub fn can_read(&self) -> bool {
        self.buffer.lock().available_read() > 0
    }
    
    /// Check if space is available to write
    pub fn can_write(&self) -> bool {
        self.buffer.lock().available_write() > 0
    }
    
    /// Get available bytes to read
    pub fn available(&self) -> usize {
        self.buffer.lock().available_read()
    }
    
    /// Close the write end
    pub fn close_write(&self) {
        self.writer_active.store(false, Ordering::Release);
        // Wake all readers (they'll get EOF)
        let mut waiters = self.read_waiters.lock();
        while waiters.pop_front().is_some() {}
    }
    
    /// Close the read end
    pub fn close_read(&self) {
        self.reader_active.store(false, Ordering::Release);
        // Wake all writers (they'll get broken pipe)
        let mut waiters = self.write_waiters.lock();
        while waiters.pop_front().is_some() {}
    }
    
    /// Check if pipe is fully closed
    pub fn is_closed(&self) -> bool {
        !self.reader_active.load(Ordering::Acquire) || !self.writer_active.load(Ordering::Acquire)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// GLOBAL IPC REGISTRY
// ═══════════════════════════════════════════════════════════════════════════════

use alloc::collections::BTreeMap;

/// Global IPC state
pub struct IpcRegistry {
    /// Named channels
    channels: Spinlock<BTreeMap<ChannelId, Arc<Channel>>>,
    /// Named channel lookup
    channel_names: Spinlock<BTreeMap<String, ChannelId>>,
    /// Pipes
    pipes: Spinlock<BTreeMap<PipeId, Arc<Pipe>>>,
    /// Next channel ID
    next_channel_id: AtomicUsize,
    /// Next pipe ID
    next_pipe_id: AtomicUsize,
}

impl IpcRegistry {
    pub const fn new() -> Self {
        Self {
            channels: Spinlock::new(BTreeMap::new()),
            channel_names: Spinlock::new(BTreeMap::new()),
            pipes: Spinlock::new(BTreeMap::new()),
            next_channel_id: AtomicUsize::new(1),
            next_pipe_id: AtomicUsize::new(1),
        }
    }
    
    /// Create a new named channel
    pub fn create_channel(&self, name: &str) -> Arc<Channel> {
        self.create_channel_with_capacity(name, DEFAULT_CHANNEL_CAPACITY)
    }
    
    /// Create a new named channel with specified capacity
    pub fn create_channel_with_capacity(&self, name: &str, capacity: usize) -> Arc<Channel> {
        let id = self.next_channel_id.fetch_add(1, Ordering::SeqCst) as ChannelId;
        let channel = Arc::new(Channel::with_capacity(id, name, capacity));
        
        self.channels.lock().insert(id, channel.clone());
        self.channel_names.lock().insert(String::from(name), id);
        
        crate::klog::klog_debug("ipc", &alloc::format!("Created channel '{}' (id={})", name, id));
        
        channel
    }
    
    /// Get a channel by name
    pub fn get_channel_by_name(&self, name: &str) -> Option<Arc<Channel>> {
        let id = *self.channel_names.lock().get(name)?;
        self.channels.lock().get(&id).cloned()
    }
    
    /// Get a channel by ID
    pub fn get_channel(&self, id: ChannelId) -> Option<Arc<Channel>> {
        self.channels.lock().get(&id).cloned()
    }
    
    /// Remove a channel
    pub fn remove_channel(&self, id: ChannelId) {
        if let Some(channel) = self.channels.lock().remove(&id) {
            self.channel_names.lock().remove(&channel.name);
            channel.close();
            crate::klog::klog_debug("ipc", &alloc::format!("Removed channel '{}' (id={})", channel.name, id));
        }
    }
    
    /// Create a new pipe
    pub fn create_pipe(&self) -> Arc<Pipe> {
        let id = self.next_pipe_id.fetch_add(1, Ordering::SeqCst) as PipeId;
        let pipe = Arc::new(Pipe::new(id));
        
        self.pipes.lock().insert(id, pipe.clone());
        
        crate::klog::klog_debug("ipc", &alloc::format!("Created pipe (id={})", id));
        
        pipe
    }
    
    /// Get a pipe by ID
    pub fn get_pipe(&self, id: PipeId) -> Option<Arc<Pipe>> {
        self.pipes.lock().get(&id).cloned()
    }
    
    /// Remove a pipe
    pub fn remove_pipe(&self, id: PipeId) {
        if let Some(pipe) = self.pipes.lock().remove(&id) {
            pipe.close_read();
            pipe.close_write();
            crate::klog::klog_debug("ipc", &alloc::format!("Removed pipe (id={})", id));
        }
    }
    
    /// List all channels
    pub fn list_channels(&self) -> Vec<(ChannelId, String, usize)> {
        self.channels
            .lock()
            .values()
            .map(|ch| (ch.id, ch.name.clone(), ch.pending_count()))
            .collect()
    }
    
    /// List all pipes
    pub fn list_pipes(&self) -> Vec<(PipeId, usize, bool)> {
        self.pipes
            .lock()
            .values()
            .map(|p| (p.id, p.available(), p.is_closed()))
            .collect()
    }
    
    /// Clean up closed channels and pipes
    pub fn cleanup(&self) -> (usize, usize) {
        let mut channels_removed = 0;
        let mut pipes_removed = 0;
        
        // Clean up closed channels
        let closed_channels: Vec<ChannelId> = self.channels
            .lock()
            .iter()
            .filter(|(_, ch)| ch.is_closed())
            .map(|(id, _)| *id)
            .collect();
        
        for id in closed_channels {
            self.remove_channel(id);
            channels_removed += 1;
        }
        
        // Clean up closed pipes
        let closed_pipes: Vec<PipeId> = self.pipes
            .lock()
            .iter()
            .filter(|(_, p)| p.is_closed())
            .map(|(id, _)| *id)
            .collect();
        
        for id in closed_pipes {
            self.remove_pipe(id);
            pipes_removed += 1;
        }
        
        (channels_removed, pipes_removed)
    }
}

/// Global IPC registry
pub static IPC: IpcRegistry = IpcRegistry::new();

