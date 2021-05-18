use crate::prelude::*;
use conquer_once::noblock;

/// A wrapper around `noblock::OnceCell` which panics immediately when error detected.
#[derive(Debug)]
pub(crate) struct OnceCell<T>(noblock::OnceCell<T>);

impl<T> OnceCell<T> {
    pub(crate) const fn uninit() -> Self {
        Self(noblock::OnceCell::uninit())
    }

    #[track_caller]
    pub(crate) fn init_once(&self, f: impl FnOnce() -> T) {
        #[allow(clippy::unwrap_used)]
        self.try_init_once(f).unwrap()
    }

    #[track_caller]
    pub(crate) fn try_init_once(&self, f: impl FnOnce() -> T) -> Result<()> {
        Ok(self.0.try_init_once(f)?)
    }

    #[track_caller]
    pub(crate) fn get(&self) -> &T {
        #[allow(clippy::unwrap_used)]
        self.try_get().unwrap()
    }

    #[track_caller]
    pub(crate) fn try_get(&self) -> Result<&T> {
        Ok(self.0.try_get()?)
    }
}
