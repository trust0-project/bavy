//! Multi-hart task scheduler
//!
//! Provides a simple priority-based scheduler that distributes tasks across
//! available harts. Features:
//! - Per-hart run queues
//! - Priority-based scheduling
//! - Work stealing (idle harts can take work from busy ones)
//! - Hart affinity support

use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering, fence};

use crate::task::{Task, TaskEntry, TaskInfo, TaskState, Pid, Priority};
use crate::Spinlock;
use crate::MAX_HARTS;

/// Per-hart run queue
pub struct RunQueue {
    /// Tasks waiting to run (priority sorted)
    tasks: VecDeque<Arc<Task>>,
    /// Currently running task on this hart
    current: Option<Arc<Task>>,
}

impl RunQueue {
    pub const fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            current: None,
        }
    }
    
    /// Add a task to the queue (maintains priority order)
    pub fn enqueue(&mut self, task: Arc<Task>) {
        // Insert based on priority (higher priority = earlier in queue)
        let priority = task.priority;
        let mut insert_pos = self.tasks.len();
        for (i, t) in self.tasks.iter().enumerate() {
            if t.priority < priority {
                insert_pos = i;
                break;
            }
        }
        self.tasks.insert(insert_pos, task);
    }
    
    /// Get next task to run
    pub fn dequeue(&mut self) -> Option<Arc<Task>> {
        // Find first runnable task
        for i in 0..self.tasks.len() {
            if self.tasks[i].is_runnable() {
                return self.tasks.remove(i);
            }
        }
        None
    }
    
    /// Number of tasks in queue
    pub fn len(&self) -> usize {
        self.tasks.len()
    }
    
    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
    
    /// Steal a task from this queue (for work stealing)
    pub fn steal(&mut self) -> Option<Arc<Task>> {
        // Steal lowest priority task from back of queue
        if self.tasks.len() > 1 {
            self.tasks.pop_back()
        } else {
            None
        }
    }
}

/// Global scheduler
pub struct Scheduler {
    /// Per-hart run queues
    queues: [Spinlock<RunQueue>; MAX_HARTS],
    /// Global task registry (PID -> Task)
    tasks: Spinlock<BTreeMap<Pid, Arc<Task>>>,
    /// Next PID to assign
    next_pid: AtomicUsize,
    /// Number of active harts
    num_harts: AtomicUsize,
    /// Scheduler is initialized and running
    running: AtomicBool,
}

// Create array of spinlocks for run queues
const fn create_queue_array() -> [Spinlock<RunQueue>; MAX_HARTS] {
    const INIT: Spinlock<RunQueue> = Spinlock::new(RunQueue {
        tasks: VecDeque::new(),
        current: None,
    });
    [INIT; MAX_HARTS]
}

/// Global scheduler instance
pub static SCHEDULER: Scheduler = Scheduler {
    queues: create_queue_array(),
    tasks: Spinlock::new(BTreeMap::new()),
    next_pid: AtomicUsize::new(1), // PID 0 is reserved (idle)
    num_harts: AtomicUsize::new(1),
    running: AtomicBool::new(false),
};

impl Scheduler {
    /// Initialize the scheduler with the number of available harts
    pub fn init(&self, num_harts: usize) {
        self.num_harts.store(num_harts, Ordering::Release);
        self.running.store(true, Ordering::Release);
        fence(Ordering::SeqCst);
        
        crate::klog::klog_info("sched", &alloc::format!("Scheduler initialized with {} harts", num_harts));
    }
    
    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }
    
    /// Spawn a new task
    pub fn spawn(&self, name: &str, entry: TaskEntry, priority: Priority) -> Pid {
        self.spawn_on_hart(name, entry, priority, None)
    }
    
    /// Spawn a task with hart affinity
    pub fn spawn_on_hart(
        &self,
        name: &str,
        entry: TaskEntry,
        priority: Priority,
        hart_affinity: Option<usize>,
    ) -> Pid {
        let pid = self.next_pid.fetch_add(1, Ordering::SeqCst) as Pid;
        let mut task = Task::new(pid, name, entry, priority);
        task.hart_affinity = hart_affinity;
        
        let task = Arc::new(task);
        
        // Register in global task table
        self.tasks.lock().insert(pid, task.clone());
        
        // Determine target hart
        let target_hart = hart_affinity.unwrap_or_else(|| self.find_least_loaded_hart());
        
        // Enqueue the task
        self.queues[target_hart].lock().enqueue(task);
        
        crate::klog::klog_debug("sched", &alloc::format!(
            "Spawned task '{}' (PID {}) on hart {}", name, pid, target_hart
        ));
        
        // Wake the target hart if not primary
        if target_hart != 0 {
            crate::send_ipi(target_hart);
        }
        
        pid
    }
    
    /// Spawn a daemon task (long-running service)
    pub fn spawn_daemon(&self, name: &str, entry: TaskEntry, priority: Priority) -> Pid {
        self.spawn_daemon_on_hart(name, entry, priority, None)
    }
    
    /// Spawn a daemon task on a specific hart
    pub fn spawn_daemon_on_hart(
        &self,
        name: &str,
        entry: TaskEntry,
        priority: Priority,
        hart_affinity: Option<usize>,
    ) -> Pid {
        let pid = self.next_pid.fetch_add(1, Ordering::SeqCst) as Pid;
        let mut task = Task::new_daemon(pid, name, entry, priority);
        task.hart_affinity = hart_affinity;
        
        let task = Arc::new(task);
        
        self.tasks.lock().insert(pid, task.clone());
        
        let target_hart = hart_affinity.unwrap_or_else(|| self.find_least_loaded_hart());
        self.queues[target_hart].lock().enqueue(task);
        
        crate::klog::klog_debug("sched", &alloc::format!(
            "Spawned daemon '{}' (PID {}) on hart {}", name, pid, target_hart
        ));
        
        pid
    }
    
    /// Find the hart with the fewest queued tasks
    fn find_least_loaded_hart(&self) -> usize {
        let num_harts = self.num_harts.load(Ordering::Relaxed);
        let mut min_load = usize::MAX;
        let mut target = 0;
        
        for hart in 0..num_harts {
            let load = self.queues[hart].lock().len();
            if load < min_load {
                min_load = load;
                target = hart;
            }
        }
        
        target
    }
    
    /// Pick the next task for a hart to run
    pub fn pick_next(&self, hart_id: usize) -> Option<Arc<Task>> {
        // First try our own queue
        if let Some(task) = self.queues[hart_id].lock().dequeue() {
            return Some(task);
        }
        
        // Try work stealing from other harts
        let num_harts = self.num_harts.load(Ordering::Relaxed);
        for other in 0..num_harts {
            if other != hart_id {
                if let Some(task) = self.queues[other].lock().steal() {
                    crate::klog::klog_trace("sched", &alloc::format!(
                        "Hart {} stole task '{}' from hart {}", 
                        hart_id, task.name, other
                    ));
                    return Some(task);
                }
            }
        }
        
        None
    }
    
    /// Requeue a task (e.g., after time slice expires)
    pub fn requeue(&self, task: Arc<Task>, hart_id: usize) {
        task.set_state(TaskState::Ready);
        self.queues[hart_id].lock().enqueue(task);
    }
    
    /// Mark a task as finished
    pub fn finish_task(&self, pid: Pid, exit_code: usize) {
        if let Some(task) = self.tasks.lock().get(&pid) {
            task.mark_finished(exit_code);
            
            crate::klog::klog_info("sched", &alloc::format!(
                "Task '{}' (PID {}) exited with code {}", task.name, pid, exit_code
            ));
            
            // If daemon with restart_on_exit, respawn it
            if task.is_daemon && task.restart_on_exit {
                let name = task.name.clone();
                let entry = task.entry;
                let priority = task.priority;
                let affinity = task.hart_affinity;
                
                // Schedule respawn
                crate::klog::klog_info("sched", &alloc::format!(
                    "Respawning daemon '{}'", name
                ));
                self.spawn_on_hart(&name, entry, priority, affinity);
            }
        }
    }
    
    /// Clean up zombie tasks
    pub fn reap_zombies(&self) -> usize {
        let mut tasks = self.tasks.lock();
        let zombies: Vec<Pid> = tasks
            .iter()
            .filter(|(_, t)| t.get_state() == TaskState::Zombie && !t.restart_on_exit)
            .map(|(pid, _)| *pid)
            .collect();
        
        let count = zombies.len();
        for pid in zombies {
            tasks.remove(&pid);
        }
        
        count
    }
    
    /// Get task by PID
    pub fn get_task(&self, pid: Pid) -> Option<Arc<Task>> {
        self.tasks.lock().get(&pid).cloned()
    }
    
    /// List all tasks
    pub fn list_tasks(&self) -> Vec<TaskInfo> {
        let current_time = crate::get_time_ms() as u64;
        self.tasks
            .lock()
            .values()
            .map(|t| t.info(current_time))
            .collect()
    }
    
    /// Get number of active tasks
    pub fn task_count(&self) -> usize {
        self.tasks.lock().len()
    }
    
    /// Get total tasks in all queues
    pub fn queued_count(&self) -> usize {
        let num_harts = self.num_harts.load(Ordering::Relaxed);
        let mut total = 0;
        for hart in 0..num_harts {
            total += self.queues[hart].lock().len();
        }
        total
    }
    
    /// Kill a task by PID
    pub fn kill(&self, pid: Pid) -> bool {
        if let Some(task) = self.tasks.lock().get(&pid) {
            task.mark_finished(137); // SIGKILL-like
            crate::klog::klog_info("sched", &alloc::format!(
                "Killed task '{}' (PID {})", task.name, pid
            ));
            true
        } else {
            false
        }
    }
}

