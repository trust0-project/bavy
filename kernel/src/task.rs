//! Task/Process abstraction for the kernel
//!
//! Provides Linux-like task management with:
//! - Task Control Block (TCB) similar to Linux's task_struct
//! - Task states (Ready, Running, Sleeping, Zombie)
//! - Priority levels for scheduling
//! - CPU time tracking

use alloc::string::String;
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

