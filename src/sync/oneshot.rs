use super::Mutex;

use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use futures_util::task::AtomicWaker;

pub(crate) fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner::new());
    let tx = Sender {
        inner: inner.clone(),
    };
    let rx = Receiver { inner };
    (tx, rx)
}

#[derive(Debug)]
pub(crate) struct Sender<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Sender<T> {
    pub(crate) fn send(self, value: T) {
        *self.inner.value.spin_lock() = Some(value);
        self.inner.waker.wake();
    }
}

#[derive(Debug)]
pub(crate) struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Future for Receiver<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // fast path
        if let Some(value) = self.inner.value.spin_lock().take() {
            return Poll::Ready(value);
        }

        self.inner.waker.register(&cx.waker());
        if let Some(value) = self.inner.value.spin_lock().take() {
            self.inner.waker.take();
            Poll::Ready(value)
        } else {
            Poll::Pending
        }
    }
}

#[derive(Debug)]
struct Inner<T> {
    value: Mutex<Option<T>>,
    waker: AtomicWaker,
}

impl<T> Inner<T> {
    fn new() -> Self {
        Self {
            value: Mutex::new(None),
            waker: AtomicWaker::new(),
        }
    }
}
