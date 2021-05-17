use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, task::Wake};
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll, Waker},
};
use crossbeam_queue::ArrayQueue;
use custom_debug_derive::Debug as CustomDebug;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CoTaskId(u64);

impl CoTaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Cooperative Task
#[derive(CustomDebug)]
pub(crate) struct CoTask {
    id: CoTaskId,
    #[debug(skip)]
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl CoTask {
    pub(crate) fn new(future: impl Future<Output = ()> + 'static) -> Self {
        Self {
            id: CoTaskId::new(),
            future: Box::pin(future),
        }
    }

    fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(cx)
    }
}

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
