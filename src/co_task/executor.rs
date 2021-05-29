use super::{CoTask, CoTaskId};
use crate::task::{self, TaskId};
use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use core::task::{Context, Poll, Waker};
use crossbeam_queue::ArrayQueue;
use x86_64::instructions::interrupts;

#[derive(Debug)]
enum Event {
    Spawn(CoTask),
    Wake(CoTaskId),
}

#[derive(Debug)]
pub(crate) struct Executor {
    task_id: TaskId,
    tasks: BTreeMap<CoTaskId, CoTask>,
    task_queue: Arc<ArrayQueue<Event>>,
    waker_cache: BTreeMap<CoTaskId, Waker>,
}

impl Executor {
    pub(crate) fn new(task_id: TaskId) -> Self {
        Self {
            task_id,
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub(crate) fn handle(&self) -> Handle {
        Handle {
            task_queue: self.task_queue.clone(),
        }
    }

    pub(crate) fn spawn(&mut self, task: CoTask) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        #[allow(clippy::expect_used)]
        self.task_queue
            .push(Event::Wake(task_id))
            .expect("queue full");
    }

    pub(crate) fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    fn wake(&mut self, co_task_id: CoTaskId) {
        // destructure `self` to avoid borrow checker errors
        let Self {
            task_id,
            tasks,
            task_queue,
            waker_cache,
        } = self;

        let task = match tasks.get_mut(&co_task_id) {
            Some(task) => task,
            None => return, // task no longer exists
        };

        let waker = waker_cache
            .entry(co_task_id)
            .or_insert_with(|| CoTaskWaker::waker(*task_id, co_task_id, task_queue.clone()));
        let mut context = Context::from_waker(waker);
        if let Poll::Ready(()) = task.poll(&mut context) {
            // task done -> remove it and its cached waker
            tasks.remove(&co_task_id);
            waker_cache.remove(&co_task_id);
        }
    }

    fn run_ready_tasks(&mut self) {
        while let Some(event) = self.task_queue.pop() {
            match event {
                Event::Spawn(task) => self.spawn(task),
                Event::Wake(task_id) => self.wake(task_id),
            }
        }
    }

    fn sleep_if_idle(&self) {
        interrupts::disable();
        if self.task_queue.is_empty() {
            task::sleep(self.task_id);
        } else {
            interrupts::enable();
        }
    }
}

struct CoTaskWaker {
    task_id: TaskId,
    co_task_id: CoTaskId,
    task_queue: Arc<ArrayQueue<Event>>,
}

impl CoTaskWaker {
    fn waker(task_id: TaskId, co_task_id: CoTaskId, task_queue: Arc<ArrayQueue<Event>>) -> Waker {
        Waker::from(Arc::new(CoTaskWaker {
            task_id,
            co_task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        interrupts::without_interrupts(|| {
            #[allow(clippy::expect_used)]
            self.task_queue
                .push(Event::Wake(self.co_task_id))
                .expect("task_queue full");
            task::wake(self.task_id);
        })
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

#[derive(Debug, Clone)]
pub(crate) struct Handle {
    task_queue: Arc<ArrayQueue<Event>>,
}

impl Handle {
    pub(crate) fn spawn(&self, task: CoTask) {
        #[allow(clippy::expect_used)]
        self.task_queue
            .push(Event::Spawn(task))
            .expect("queue full");
    }
}
