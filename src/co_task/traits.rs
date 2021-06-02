use core::{
    fmt::Debug,
    future::Future,
    panic::Location,
    pin::Pin,
    task::{Context, Poll},
};
use futures_util::TryFuture;
use pin_project::pin_project;

impl<Fut: ?Sized + TryFuture> TryFutureExt for Fut {}

pub(crate) trait TryFutureExt: TryFuture {
    #[track_caller]
    fn unwrap(self) -> Unwrap<Self>
    where
        Self: Sized,
    {
        Unwrap(self, Location::caller())
    }
}

#[pin_project]
#[derive(Debug)]
pub(crate) struct Unwrap<Fut>(#[pin] Fut, &'static Location<'static>);

impl<Fut> Future for Unwrap<Fut>
where
    Fut: TryFuture,
    Fut::Error: Debug,
{
    type Output = Fut::Ok;

    #[track_caller]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.0.try_poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(value)) => Poll::Ready(value),
            Poll::Ready(Err(err)) => panic!("panic at {}: {:?}", this.1, err),
        }
    }
}
