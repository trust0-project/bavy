//! Task/Process abstraction for the kernel
//!
//! Provides Linux-like task management with:
//! - Task Control Block (TCB) similar to Linux's task_struct
//! - Task states (Ready, Running, Sleeping, Zombie)
//! - Priority levels for scheduling
//! - CPU time tracking
//! - WaitQueues for event-based blocking

use crate::Spinlock;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Process identifier type
pub type Pid = u32;

/// Task states (similar to Linux process states)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum TaskState {
    /// Task is runnable, waiting for CPU
    Ready = 0,
    /// Task is currently executing on a hart
    Running = 1,
    /// Task is blocked (sleeping, waiting for I/O)
    Sleeping = 2,
    /// Task has been stopped (can be resumed)
    Stopped = 3,
    /// Task has finished, awaiting cleanup
    Zombie = 4,
}

impl TaskState {
    pub fn from_usize(val: usize) -> Self {
        match val {
            0 => TaskState::Ready,
            1 => TaskState::Running,
            2 => TaskState::Sleeping,
            3 => TaskState::Stopped,
            _ => TaskState::Zombie,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TaskState::Ready => "R",
            TaskState::Running => "R+",
            TaskState::Sleeping => "S",
            TaskState::Stopped => "T",
            TaskState::Zombie => "Z",
        }
    }
}

/// Task priority levels for scheduling
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum Priority {
    /// Lowest priority - runs when nothing else to do
    Idle = 0,
    /// Background tasks
    Low = 1,
    /// Default priority for user tasks
    Normal = 2,
    /// System services
    High = 3,
    /// Critical system tasks
    Realtime = 4,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::Idle => "idle",
            Priority::Low => "low",
            Priority::Normal => "normal",
            Priority::High => "high",
            Priority::Realtime => "rt",
        }
    }
}

/// Task entry point function type
/// The function receives a reference to its own task and any user data
pub type TaskEntry = fn();

/// Task Control Block - represents a schedulable unit of execution
pub struct Task {
    /// Unique process identifier
    pub pid: Pid,
    /// Human-readable task name
    pub name: String,
    /// Current task state (atomic for cross-hart visibility)
    state: AtomicUsize,
    /// Task priority
    pub priority: Priority,
    /// Hart affinity (None = can run on any hart)
    pub hart_affinity: Option<usize>,
    /// Hart currently running this task (if Running)
    pub current_hart: AtomicUsize,
    /// Task entry point
    pub entry: TaskEntry,
    /// Creation timestamp (ms since boot)
    pub created_at: u64,
    /// Total CPU time consumed (ms)
    pub cpu_time: AtomicU64,
    /// Exit code (valid when Zombie)
    pub exit_code: AtomicUsize,
    /// Whether this is a daemon (long-running service)
    pub is_daemon: bool,
    /// Whether task should restart on exit
    pub restart_on_exit: bool,
}

impl Task {
    /// Create a new task
    pub fn new(pid: Pid, name: &str, entry: TaskEntry, priority: Priority) -> Self {
        Self {
            pid,
            name: String::from(name),
            state: AtomicUsize::new(TaskState::Ready as usize),
            priority,
            hart_affinity: None,
            current_hart: AtomicUsize::new(usize::MAX),
            entry,
            created_at: crate::get_time_ms() as u64,
            cpu_time: AtomicU64::new(0),
            exit_code: AtomicUsize::new(0),
            is_daemon: false,
            restart_on_exit: false,
        }
    }

    /// Create a daemon task (long-running service)
    pub fn new_daemon(pid: Pid, name: &str, entry: TaskEntry, priority: Priority) -> Self {
        let mut task = Self::new(pid, name, entry, priority);
        task.is_daemon = true;
        task.restart_on_exit = true;
        task
    }

    /// Get current task state
    pub fn get_state(&self) -> TaskState {
        TaskState::from_usize(self.state.load(Ordering::Acquire))
    }

    /// Set task state
    pub fn set_state(&self, state: TaskState) {
        self.state.store(state as usize, Ordering::Release);
    }

    /// Check if task is runnable
    pub fn is_runnable(&self) -> bool {
        matches!(self.get_state(), TaskState::Ready)
    }

    /// Mark task as running on specified hart
    pub fn mark_running(&self, hart_id: usize) {
        self.current_hart.store(hart_id, Ordering::Release);
        self.set_state(TaskState::Running);
    }

    /// Mark task as finished with exit code
    pub fn mark_finished(&self, exit_code: usize) {
        self.exit_code.store(exit_code, Ordering::Release);
        self.current_hart.store(usize::MAX, Ordering::Release);
        self.set_state(TaskState::Zombie);
    }

    /// Add CPU time
    pub fn add_cpu_time(&self, ms: u64) {
        self.cpu_time.fetch_add(ms, Ordering::Relaxed);
    }

    /// Get total CPU time consumed
    pub fn get_cpu_time(&self) -> u64 {
        self.cpu_time.load(Ordering::Relaxed)
    }

    /// Get current hart (if running)
    pub fn get_current_hart(&self) -> Option<usize> {
        let hart = self.current_hart.load(Ordering::Acquire);
        if hart == usize::MAX {
            None
        } else {
            Some(hart)
        }
    }
}

/// Task information for reporting (does not hold references)
#[derive(Clone)]
pub struct TaskInfo {
    pub pid: Pid,
    pub name: String,
    pub state: TaskState,
    pub priority: Priority,
    pub hart: Option<usize>,
    pub cpu_time: u64,
    pub uptime: u64,
}

impl Task {
    /// Get a snapshot of task info for reporting
    pub fn info(&self, current_time: u64) -> TaskInfo {
        TaskInfo {
            pid: self.pid,
            name: self.name.clone(),
            state: self.get_state(),
            priority: self.priority,
            hart: self.get_current_hart(),
            cpu_time: self.get_cpu_time(),
            uptime: current_time.saturating_sub(self.created_at),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// WAIT QUEUE - Event-based task blocking
// ═══════════════════════════════════════════════════════════════════════════════

/// Event types that tasks can wait on
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum WaitEvent {
    /// Timer expired
    Timer = 0,
    /// I/O available (network, block device)
    IoReady = 1,
    /// IPC message available
    IpcMessage = 2,
    /// Child process exited
    ChildExit = 3,
    /// Generic signal
    Signal = 4,
}

/// A waiter entry in a wait queue
#[derive(Clone)]
pub struct Waiter {
    /// PID of the waiting task
    pub pid: Pid,
    /// Event being waited on
    pub event: WaitEvent,
    /// Optional timeout (absolute timestamp in ms)
    pub timeout: Option<u64>,
    /// Data associated with the wait (e.g., channel ID for IPC)
    pub data: u64,
}

/// A wait queue for blocking tasks until an event occurs
pub struct WaitQueue {
    /// Name for debugging
    name: String,
    /// Waiters in FIFO order
    waiters: Spinlock<VecDeque<Waiter>>,
    /// Number of wake signals pending
    pending_wakes: AtomicUsize,
}

impl WaitQueue {
    /// Create a new wait queue
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            waiters: Spinlock::new(VecDeque::new()),
            pending_wakes: AtomicUsize::new(0),
        }
    }

    /// Add a task to the wait queue
    pub fn wait(&self, pid: Pid, event: WaitEvent, timeout: Option<u64>, data: u64) {
        let waiter = Waiter {
            pid,
            event,
            timeout,
            data,
        };
        self.waiters.lock().push_back(waiter);
        crate::klog::klog_trace(
            "waitq",
            &alloc::format!("Task {} waiting on {:?} (queue={})", pid, event, self.name),
        );
    }

    /// Wake one waiter (FIFO order)
    /// Returns the PID of the woken task, if any
    pub fn wake_one(&self) -> Option<Pid> {
        let waiter = self.waiters.lock().pop_front()?;
        crate::klog::klog_trace(
            "waitq",
            &alloc::format!(
                "Waking task {} from {:?} (queue={})",
                waiter.pid,
                waiter.event,
                self.name
            ),
        );

        // Mark the task as ready
        if let Some(task) = crate::scheduler::SCHEDULER.get_task(waiter.pid) {
            task.set_state(TaskState::Ready);
        }

        Some(waiter.pid)
    }

    /// Wake all waiters
    /// Returns the number of tasks woken
    pub fn wake_all(&self) -> usize {
        let mut count = 0;
        let mut waiters = self.waiters.lock();
        while let Some(waiter) = waiters.pop_front() {
            crate::klog::klog_trace(
                "waitq",
                &alloc::format!(
                    "Waking task {} from {:?} (queue={})",
                    waiter.pid,
                    waiter.event,
                    self.name
                ),
            );
            if let Some(task) = crate::scheduler::SCHEDULER.get_task(waiter.pid) {
                task.set_state(TaskState::Ready);
            }
            count += 1;
        }
        count
    }

    /// Wake waiters matching a specific event type
    pub fn wake_event(&self, event: WaitEvent) -> usize {
        let mut count = 0;
        let mut waiters = self.waiters.lock();
        let mut remaining = VecDeque::new();

        while let Some(waiter) = waiters.pop_front() {
            if waiter.event == event {
                crate::klog::klog_trace(
                    "waitq",
                    &alloc::format!(
                        "Waking task {} from {:?} (queue={})",
                        waiter.pid,
                        waiter.event,
                        self.name
                    ),
                );
                if let Some(task) = crate::scheduler::SCHEDULER.get_task(waiter.pid) {
                    task.set_state(TaskState::Ready);
                }
                count += 1;
            } else {
                remaining.push_back(waiter);
            }
        }

        *waiters = remaining;
        count
    }

    /// Check for and remove timed-out waiters
    /// Returns PIDs of timed-out tasks
    pub fn check_timeouts(&self, current_time: u64) -> Vec<Pid> {
        let mut timed_out = Vec::new();
        let mut waiters = self.waiters.lock();
        let mut remaining = VecDeque::new();

        while let Some(waiter) = waiters.pop_front() {
            if let Some(timeout) = waiter.timeout {
                if current_time >= timeout {
                    crate::klog::klog_trace(
                        "waitq",
                        &alloc::format!(
                            "Task {} timed out on {:?} (queue={})",
                            waiter.pid,
                            waiter.event,
                            self.name
                        ),
                    );
                    if let Some(task) = crate::scheduler::SCHEDULER.get_task(waiter.pid) {
                        task.set_state(TaskState::Ready);
                    }
                    timed_out.push(waiter.pid);
                    continue;
                }
            }
            remaining.push_back(waiter);
        }

        *waiters = remaining;
        timed_out
    }

    /// Get number of waiters
    pub fn len(&self) -> usize {
        self.waiters.lock().len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.waiters.lock().is_empty()
    }

    /// Check if a specific PID is waiting
    pub fn is_waiting(&self, pid: Pid) -> bool {
        self.waiters.lock().iter().any(|w| w.pid == pid)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// GLOBAL WAIT QUEUES
// ═══════════════════════════════════════════════════════════════════════════════

/// Global wait queue for timer events
pub static TIMER_WAITQ: Spinlock<Option<WaitQueue>> = Spinlock::new(None);

/// Global wait queue for I/O events  
pub static IO_WAITQ: Spinlock<Option<WaitQueue>> = Spinlock::new(None);

/// Global wait queue for IPC events
pub static IPC_WAITQ: Spinlock<Option<WaitQueue>> = Spinlock::new(None);

/// Initialize global wait queues
pub fn init_wait_queues() {
    *TIMER_WAITQ.lock() = Some(WaitQueue::new("timer"));
    *IO_WAITQ.lock() = Some(WaitQueue::new("io"));
    *IPC_WAITQ.lock() = Some(WaitQueue::new("ipc"));
    crate::klog::klog_debug("waitq", "Wait queues initialized");
}

/// Helper to add a task to the timer wait queue
pub fn wait_timer(pid: Pid, timeout_ms: u64) {
    let current_time = crate::get_time_ms() as u64;
    let deadline = current_time + timeout_ms;

    if let Some(ref wq) = *TIMER_WAITQ.lock() {
        wq.wait(pid, WaitEvent::Timer, Some(deadline), 0);
    }
}

/// Helper to add a task to the I/O wait queue
pub fn wait_io(pid: Pid, timeout: Option<u64>) {
    if let Some(ref wq) = *IO_WAITQ.lock() {
        wq.wait(pid, WaitEvent::IoReady, timeout, 0);
    }
}

/// Helper to wake tasks waiting for I/O
pub fn wake_io() -> usize {
    if let Some(ref wq) = *IO_WAITQ.lock() {
        wq.wake_all()
    } else {
        0
    }
}

/// Helper to add a task to the IPC wait queue
pub fn wait_ipc(pid: Pid, channel_id: u64, timeout: Option<u64>) {
    if let Some(ref wq) = *IPC_WAITQ.lock() {
        wq.wait(pid, WaitEvent::IpcMessage, timeout, channel_id);
    }
}

/// Helper to wake tasks waiting for IPC on a specific channel
pub fn wake_ipc(channel_id: u64) -> usize {
    if let Some(ref wq) = *IPC_WAITQ.lock() {
        let mut count = 0;
        let mut waiters = wq.waiters.lock();
        let mut remaining = VecDeque::new();

        while let Some(waiter) = waiters.pop_front() {
            if waiter.event == WaitEvent::IpcMessage && waiter.data == channel_id {
                if let Some(task) = crate::scheduler::SCHEDULER.get_task(waiter.pid) {
                    task.set_state(TaskState::Ready);
                }
                count += 1;
            } else {
                remaining.push_back(waiter);
            }
        }

        *waiters = remaining;
        count
    } else {
        0
    }
}

/// Check all wait queues for timeouts (call periodically)
pub fn check_all_timeouts(current_time: u64) {
    if let Some(ref wq) = *TIMER_WAITQ.lock() {
        wq.check_timeouts(current_time);
    }
    if let Some(ref wq) = *IO_WAITQ.lock() {
        wq.check_timeouts(current_time);
    }
    if let Some(ref wq) = *IPC_WAITQ.lock() {
        wq.check_timeouts(current_time);
    }
}
