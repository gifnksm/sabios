use crate::{print, println, serial_print, serial_println};
use core::fmt;

static CONSOLE_LOG_LEVEL: spin::RwLock<Level> = spin::RwLock::new(Level::Warn);
static SERIAL_LOG_LEVEL: spin::RwLock<Level> = spin::RwLock::new(Level::Info);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        };
        write!(f, "{}", s)
    }
}

pub(crate) fn set_level(console_level: Level, serial_level: Level) {
    *CONSOLE_LOG_LEVEL.write() = console_level;
    *SERIAL_LOG_LEVEL.write() = serial_level;
}

#[doc(hidden)]
pub(crate) fn _log(
    level: Level,
    args: fmt::Arguments,
    file: &str,
    line: u32,
    cont_line: bool,
    newline: bool,
) {
    if level <= *SERIAL_LOG_LEVEL.read() {
        match (cont_line, newline) {
            (true, true) => serial_println!("{}", args),
            (true, false) => serial_print!("{}", args),
            (false, true) => serial_println!("[{}] {}:{} {}", level, file, line, args),
            (false, false) => serial_print!("[{}] {}:{} {}", level, file, line, args),
        }
    }
    if level <= *CONSOLE_LOG_LEVEL.read() {
        match (cont_line, newline) {
            (true, true) => println!("{}", args),
            (true, false) => print!("{}", args),
            (false, true) => println!("[{}] {}", level, args),
            (false, false) => print!("[{}] {}", level, args),
        }
    }
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        $crate::log::_log($level, format_args!($($arg)*), file!(), line!(), false, true);
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
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => ($crate::log!($crate::log::Level::Trace, $($arg)*));
}
