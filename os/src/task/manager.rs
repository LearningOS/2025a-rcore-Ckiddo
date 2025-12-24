//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // self.ready_queue.pop_front()
        if self.ready_queue.is_empty() {
            return None;
        }
        let min_process = self
            .ready_queue
            .iter()
            .enumerate()
            .min_by(|(_, p1), (_, p2)| {
                let s1 = p1.inner_exclusive_access().stride;
                let s2 = p2.inner_exclusive_access().stride;
                s1.wrapping_sub(s2).cmp(&(BIG_STRIDE / 2))
            });

        let (min_idx, min_process) = min_process.unwrap();
        let mut inner = min_process.inner_exclusive_access();
        inner.stride = inner.stride.wrapping_add(BIG_STRIDE / inner.priority);
        drop(inner);
        self.ready_queue.remove(min_idx)
    }
}

const BIG_STRIDE: u64 = !0u64;

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
