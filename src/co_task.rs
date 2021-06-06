use alloc::boxed::Box;
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};
use custom_debug_derive::Debug as CustomDebug;

pub(crate) use self::{executor::*, traits::*};

mod executor;
mod traits;

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
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl CoTask {
    pub(crate) fn new(future: impl Future<Output = ()> + Send + 'static) -> Self {
        Self {
            id: CoTaskId::new(),
            future: Box::pin(future),
        }
    }

    fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(cx)
    }
}
