use crate::{
    prelude::*,
    task::{self, TaskId},
};
use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};
use crossbeam_queue::SegQueue;
use x86_64::instructions::interrupts;

pub(crate) struct Mutex<T: ?Sized> {
    lock: AtomicBool,
    queue: SegQueue<TaskId>,
    data: UnsafeCell<T>,
}

pub(crate) struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a AtomicBool,
    queue: &'a SegQueue<TaskId>,
    data: &'a mut T,
}

unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    #[inline(always)]
    pub(crate) const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            queue: SegQueue::new(),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T> Mutex<T>
where
    T: ?Sized,
{
    #[inline(always)]
    pub(crate) fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    #[inline(always)]
    #[track_caller]
    pub(crate) fn try_lock(&self) -> Result<MutexGuard<T>> {
        // The reason for using a strong compare_exchange is explained here:
        // https://github.com/Amanieu/parking_lot/pull/207#issuecomment-575869107
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Ok(MutexGuard {
                lock: &self.lock,
                queue: &self.queue,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            bail!(ErrorKind::Deadlock)
        }
    }

    #[inline(always)]
    #[track_caller]
    pub(crate) fn lock(&self) -> MutexGuard<T> {
        let task_id = interrupts::without_interrupts(|| task::current().id());

        // Can fail to lock even if the lock is not locked. May be more efficient than `try_lock`
        // when called in a loop.
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.is_locked() {
                assert!(interrupts::are_enabled());
                self.queue.push(task_id);
                interrupts::without_interrupts(|| task::sleep(task_id));
            }
        }

        MutexGuard {
            lock: &self.lock,
            queue: &self.queue,
            data: unsafe { &mut *self.data.get() },
        }
    }
}

impl<T> fmt::Debug for Mutex<T>
where
    T: ?Sized + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.try_lock() {
            Ok(guard) => write!(f, "Mutex {{ data: {:?} }}", guard),
            Err(_) => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T> Default for Mutex<T>
where
    T: ?Sized + Default,
{
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<T> fmt::Debug for MutexGuard<'_, T>
where
    T: ?Sized + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> fmt::Display for MutexGuard<'_, T>
where
    T: ?Sized + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T> Deref for MutexGuard<'_, T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.data
    }
}

impl<T> DerefMut for MutexGuard<'_, T>
where
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<T> Drop for MutexGuard<'_, T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);

        let len = self.queue.len();
        let mut count = 0;
        while let Some(task_id) = self.queue.pop() {
            interrupts::without_interrupts(|| task::wake(task_id));
            count += 1;
            if count >= len {
                break;
            }
        }
    }
}
