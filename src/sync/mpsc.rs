use crate::prelude::*;
use alloc::sync::Arc;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream};

pub(crate) fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner::new(buffer));
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
    pub(crate) fn send(&self, value: T) -> Result<()> {
        self.inner.queue.push(value).map_err(|_| ErrorKind::Full)?;
        self.inner.waker.wake();
        Ok(())
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // fast path
        if let Some(value) = self.inner.queue.pop() {
            return Poll::Ready(Some(value));
        }

        self.inner.waker.register(cx.waker());
        if let Some(value) = self.inner.queue.pop() {
            self.inner.waker.take();
            Poll::Ready(Some(value))
        } else {
            Poll::Pending
        }
    }
}

#[derive(Debug)]
struct Inner<T> {
    queue: ArrayQueue<T>,
    waker: AtomicWaker,
}

impl<T> Inner<T> {
    fn new(buffer: usize) -> Self {
        Self {
            queue: ArrayQueue::new(buffer),
            waker: AtomicWaker::new(),
        }
    }
}
