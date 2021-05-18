use crate::prelude::*;

/// A wrapper around `spin::Mutex` which panics immediately when deadlock detected.
#[derive(Debug, Default)]
pub(crate) struct Mutex<T: ?Sized>(spin::Mutex<T>);

pub(crate) type MutexGuard<'a, T> = spin::MutexGuard<'a, T>;

impl<T> Mutex<T> {
    pub(crate) const fn new(value: T) -> Self {
        Self(spin::Mutex::new(value))
    }
}

impl<T> Mutex<T>
where
    T: ?Sized,
{
    #[track_caller]
    pub(crate) fn lock(&self) -> MutexGuard<'_, T> {
        #[allow(clippy::unwrap_used)]
        self.try_lock().unwrap()
    }

    #[track_caller]
    pub(crate) fn try_lock(&self) -> Result<MutexGuard<'_, T>> {
        Ok(self.0.try_lock().ok_or(ErrorKind::Deadlock)?)
    }

    pub(crate) unsafe fn force_unlock(&self) {
        unsafe { self.0.force_unlock() }
    }
}
