use crate::{
    error::ConvertErr as _,
    prelude::*,
    sync::mutex::{Mutex, MutexGuard},
};
use conquer_once::noblock::OnceCell;

#[track_caller]
pub(crate) fn try_get_and_lock<T>(
    mutex: &'static OnceCell<Mutex<T>>,
    target: &'static str,
) -> Result<MutexGuard<'static, T>> {
    Ok(mutex.try_get().convert_err(target)?.lock())
}
