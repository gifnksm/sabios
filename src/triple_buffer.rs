use crate::sync::{SpinMutex, SpinMutexGuard};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

pub(crate) fn new<T>(buffer: T) -> (Producer<T>, Consumer<T>)
where
    T: Clone,
{
    let inner = Arc::new(Inner {
        state: AtomicU64::new(1),
        buffers: [
            // present
            Buffer::new(0, buffer.clone()),
            // ready
            Buffer::new(0, buffer.clone()),
            // in_progress
            Buffer::new(1, buffer),
        ],
    });
    let cons = Consumer {
        swap_state: state_new(1, 0),
        present: 0,
        inner: inner.clone(),
    };
    let prod = Producer {
        in_progress: 2,
        inner,
    };
    (prod, cons)
}

#[derive(Debug)]
pub(crate) struct Producer<T> {
    in_progress: usize,
    inner: Arc<Inner<T>>,
}

impl<T> Producer<T> {
    #[cfg(test)]
    fn epoch(&self) -> u64 {
        self.inner.buffers[self.in_progress]
            .epoch
            .load(Ordering::Relaxed)
    }

    #[cfg(test)]
    pub(crate) fn buffer(&self) -> SpinMutexGuard<T> {
        self.inner.buffers[self.in_progress].buffer()
    }

    pub(crate) fn with_buffer<U>(&self, f: impl FnOnce(&mut T) -> U) -> U {
        self.inner.buffers[self.in_progress].with_buffer(f)
    }

    pub(crate) fn store(&mut self) {
        // swap in-progress buffer and ready buffer
        let swap_epoch = self.inner.buffers[self.in_progress]
            .epoch
            .load(Ordering::Relaxed);

        let swap_state = state_new(self.in_progress, swap_epoch);
        let old_state = self.inner.state.swap(swap_state, Ordering::SeqCst);

        // update new in-progress buffer index & epoch
        self.in_progress = state_to_index(old_state);
        let mut new_epoch = swap_epoch + 1;
        if new_epoch > (u64::MAX >> 2) {
            new_epoch = 0;
        }
        self.inner.buffers[self.in_progress]
            .epoch
            .store(new_epoch, Ordering::Relaxed);
    }
}

#[derive(Debug)]
pub(crate) struct Consumer<T> {
    swap_state: u64,
    present: usize,
    inner: Arc<Inner<T>>,
}

impl<T> Consumer<T> {
    #[cfg(test)]
    fn epoch(&self) -> u64 {
        self.inner.buffers[self.present]
            .epoch
            .load(Ordering::Relaxed)
    }

    pub(crate) fn buffer(&self) -> SpinMutexGuard<T> {
        self.inner.buffers[self.present].buffer()
    }

    pub(crate) fn load(&mut self) {
        // swap present buffer and ready buffer if ready buffer has been updated
        let epoch = self.inner.buffers[self.present]
            .epoch
            .load(Ordering::Relaxed);
        let swap_state = state_new(self.present, epoch);
        assert_ne!(self.swap_state, swap_state);
        if let Ok(old_state) = exchange_if_ne(&self.inner.state, self.swap_state, swap_state) {
            self.swap_state = swap_state;
            self.present = state_to_index(old_state)
        }
    }
}

fn exchange_if_ne(target: &AtomicU64, comp: u64, new: u64) -> Result<u64, u64> {
    let mut current = target.load(Ordering::SeqCst);
    while current != comp {
        match target.compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => return Ok(current),
            Err(value) => current = value,
        }
    }
    Err(current)
}

fn state_new(index: usize, epoch: u64) -> u64 {
    assert!(index < 3);
    assert!(epoch <= (u64::MAX >> 2));
    (index as u64) | (epoch << 2)
}

fn state_to_index(state: u64) -> usize {
    (state & 0b11) as usize
}

#[derive(Debug)]
struct Inner<T> {
    state: AtomicU64,
    buffers: [Buffer<T>; 3],
}

#[derive(Debug)]
struct Buffer<T> {
    epoch: AtomicU64,
    value: SpinMutex<T>,
}

impl<T> Buffer<T> {
    fn new(epoch: u64, value: T) -> Self {
        Self {
            epoch: AtomicU64::new(epoch),
            value: SpinMutex::new(value),
        }
    }

    fn buffer(&self) -> SpinMutexGuard<T> {
        self.value.lock()
    }

    fn with_buffer<U>(&self, f: impl FnOnce(&mut T) -> U) -> U {
        self.value.with_lock(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[track_caller]
    fn check_epoch<T>(
        prod: &Producer<T>,
        cons: &Consumer<T>,
        present: u64,
        ready: u64,
        in_progress: u64,
    ) {
        assert_eq!(cons.epoch(), present);
        let inner = &cons.inner;
        let ready_idx = state_to_index(inner.state.load(Ordering::Relaxed));
        assert_eq!(
            inner.buffers[ready_idx].epoch.load(Ordering::Relaxed),
            ready
        );
        assert_eq!(prod.epoch(), in_progress);
    }

    #[test_case]
    fn basic() {
        let buffer = vec![1, 2, 3];
        let (mut prod, mut cons) = new(buffer);
        check_epoch(&prod, &cons, 0, 0, 1);

        // Load initial value
        cons.load();
        assert_eq!(&*cons.buffer(), &[1, 2, 3]);
        check_epoch(&prod, &cons, 0, 0, 1);

        // Store updated value
        prod.buffer().clear();
        prod.buffer().extend_from_slice(&[4, 5, 6]);
        prod.store();
        check_epoch(&prod, &cons, 0, 1, 2);

        // Load updated value
        cons.load();
        assert_eq!(&*cons.buffer(), &[4, 5, 6]);
        check_epoch(&prod, &cons, 1, 0, 2);
    }

    #[test_case]
    fn faster_consumer() {
        let buffer = vec![1, 2, 3];
        let (mut prod, mut cons) = new(buffer);

        // Store updated value
        *prod.buffer() = vec![4, 5, 6];
        prod.store();
        check_epoch(&prod, &cons, 0, 1, 2);

        // Load updated value
        cons.load();
        assert_eq!(&*cons.buffer(), &[4, 5, 6]);
        check_epoch(&prod, &cons, 1, 0, 2);

        // Load again (do nothing)
        cons.load();
        assert_eq!(&*cons.buffer(), &[4, 5, 6]);
        check_epoch(&prod, &cons, 1, 0, 2);

        // Store updated value
        *prod.buffer() = vec![7, 8, 9];
        prod.store();
        check_epoch(&prod, &cons, 1, 2, 3);

        // Load updated value
        cons.load();
        assert_eq!(&*cons.buffer(), &[7, 8, 9]);
        check_epoch(&prod, &cons, 2, 1, 3);

        // Load again (do nothing)
        cons.load();
        assert_eq!(&*cons.buffer(), &[7, 8, 9]);
        check_epoch(&prod, &cons, 2, 1, 3);

        // Load once more again (do nothing)
        cons.load();
        assert_eq!(&*cons.buffer(), &[7, 8, 9]);
        check_epoch(&prod, &cons, 2, 1, 3);
    }

    #[test_case]
    fn faster_producer() {
        let buffer = vec![1, 2, 3];
        let (mut prod, mut cons) = new(buffer);

        // Store updated value
        *prod.buffer() = vec![4, 5, 6];
        prod.store();
        check_epoch(&prod, &cons, 0, 1, 2);

        // Store updated value again
        *prod.buffer() = vec![7, 8, 9];
        prod.store();
        check_epoch(&prod, &cons, 0, 2, 3);

        // Load updated value
        cons.load();
        assert_eq!(&*cons.buffer(), &[7, 8, 9]);
        check_epoch(&prod, &cons, 2, 0, 3);

        // Store updated value
        *prod.buffer() = vec![13, 14, 15];
        prod.store();
        check_epoch(&prod, &cons, 2, 3, 4);

        // Store updated value again
        *prod.buffer() = vec![16, 17, 18];
        prod.store();
        check_epoch(&prod, &cons, 2, 4, 5);

        // Store updated value once more again
        *prod.buffer() = vec![19, 20, 21];
        prod.store();
        check_epoch(&prod, &cons, 2, 5, 6);

        // Load updated value
        cons.load();
        assert_eq!(&*cons.buffer(), &[19, 20, 21]);
        check_epoch(&prod, &cons, 5, 2, 6);
    }
}
