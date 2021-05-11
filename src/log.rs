use core::fmt;

static LOG_LEVEL: spin::RwLock<Level> = spin::RwLock::new(Level::Warn);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum Level {
    Error = 3,
    Warn = 4,
    Info = 6,
    Debug = 7,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
        };
        write!(f, "{}", s)
    }
}

pub(crate) fn check_level(level: Level) -> bool {
    level <= *LOG_LEVEL.read()
}

pub(crate) fn set_level(level: Level) {
    *LOG_LEVEL.write() = level;
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        if $crate::log::check_level($level) {
            $crate::println!("[{}] {}", $level, format_args!($($arg)*));
        }
    }
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::log!($crate::log::Level::Error, $($arg)*));
}
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::log!($crate::log::Level::Warn, $($arg)*));
}
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::log!($crate::log::Level::Info, $($arg)*));
}
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::log!($crate::log::Level::Debug, $($arg)*));
}
