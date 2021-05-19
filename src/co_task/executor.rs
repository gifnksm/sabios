use super::{CoTask, CoTaskId};
use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use core::task::{Context, Poll, Waker};
use crossbeam_queue::ArrayQueue;

#[derive(Debug)]
pub(crate) struct Executor {
    tasks: BTreeMap<CoTaskId, CoTask>,
    task_queue: Arc<ArrayQueue<CoTaskId>>,
    waker_cache: BTreeMap<CoTaskId, Waker>,
}

impl Executor {
    pub(crate) fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub(crate) fn spawn(&mut self, task: CoTask) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        #[allow(clippy::expect_used)]
        self.task_queue.push(task_id).expect("queue full");
    }

    pub(crate) fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    fn run_ready_tasks(&mut self) {
        // destructure `self` to avoid borrow checker errors
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        while let Some(task_id) = task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue, // task no longer exists
            };
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| CoTaskWaker::waker(task_id, task_queue.clone()));
            let mut context = Context::from_waker(waker);
            if let Poll::Ready(()) = task.poll(&mut context) {
                // task done -> remove it and its cached waker
                tasks.remove(&task_id);
                waker_cache.remove(&task_id);
            }
        }
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_and_hlt};

        interrupts::disable();
        if self.task_queue.is_empty() {
            enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }
}

struct CoTaskWaker {
    task_id: CoTaskId,
    task_queue: Arc<ArrayQueue<CoTaskId>>,
}

impl CoTaskWaker {
    fn waker(task_id: CoTaskId, task_queue: Arc<ArrayQueue<CoTaskId>>) -> Waker {
        Waker::from(Arc::new(CoTaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        #[allow(clippy::expect_used)]
        self.task_queue.push(self.task_id).expect("task_queue full")
    }
}

impl Wake for CoTaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task()
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task()
    }
}
