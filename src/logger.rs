use core::sync::atomic::{AtomicUsize, Ordering};

pub const ERROR: usize = 1;
pub const WARN: usize = 2;
pub const INFO: usize = 3;
pub const DEBUG: usize = 4;
pub const TRACE: usize = 5;

static MAX_LEVEL: AtomicUsize = AtomicUsize::new(INFO);

pub fn set_level(level: usize) {
    MAX_LEVEL.store(level, Ordering::Relaxed);
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => { $crate::logger::log($crate::logger::ERROR, 1, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { $crate::logger::log($crate::logger::WARN, 1, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { $crate::logger::log($crate::logger::INFO, 1, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => { $crate::logger::log($crate::logger::DEBUG, 1, format_args!($($arg)*)) };
}

pub fn log(level: usize, _target: u8, args: core::fmt::Arguments) {
    if level <= MAX_LEVEL.load(Ordering::Relaxed) {
        let prefix = match level {
            ERROR => "[ERR ]",
            WARN  => "[WARN]",
            INFO  => "[INFO]",
            DEBUG => "[DBG ]",
            TRACE => "[TRC ]",
            _     => "[??? ]",
        };
        crate::serial::print(format_args!("{} {}\n", prefix, args));
        crate::vga::print(format_args!("{} {}\n", prefix, args));
    }
}
