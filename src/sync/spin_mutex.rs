use crate::prelude::*;

/// A wrapper around `spin::Mutex` which panics immediately when deadlock detected.
#[derive(Debug, Default)]
pub(crate) struct SpinMutex<T: ?Sized>(spin::Mutex<T>);

pub(crate) type SpinMutexGuard<'a, T> = spin::MutexGuard<'a, T>;

impl<T> SpinMutex<T> {
    pub(crate) const fn new(value: T) -> Self {
        Self(spin::Mutex::new(value))
    }
}

impl<T> SpinMutex<T>
where
    T: ?Sized,
{
    #[track_caller]
    pub(crate) fn lock(&self) -> SpinMutexGuard<'_, T> {
        #[allow(clippy::unwrap_used)]
        self.try_lock().unwrap()
    }

    #[track_caller]
    pub(crate) fn try_lock(&self) -> Result<SpinMutexGuard<'_, T>> {
        Ok(self.0.try_lock().ok_or(ErrorKind::Deadlock)?)
    }

    pub(crate) unsafe fn force_unlock(&self) {
        unsafe { self.0.force_unlock() }
    }

    #[track_caller]
    pub(crate) fn with_lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut *self.lock())
    }
}
