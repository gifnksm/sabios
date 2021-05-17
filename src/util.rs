use crate::{error::ConvertErr as _, prelude::*};
use conquer_once::spin::OnceCell;

#[track_caller]
pub(crate) fn try_get_and_lock<T>(
    mutex: &'static OnceCell<spin::Mutex<T>>,
    target: &'static str,
) -> Result<spin::MutexGuard<'static, T>> {
    Ok(mutex
        .try_get()
        .convert_err(target)?
        .try_lock()
        .ok_or(ErrorKind::Deadlock(target))?)
}
