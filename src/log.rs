use core::fmt::Arguments;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering::Relaxed;

/// A log level supported by the kernel.
///
/// Depending on the configuration of the kernel, some log levels may be ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Trace,
    Info,
    Warn,
    Error,
}

/// A function that can be used to log messages.
pub type LogFn = fn(lvl: Level, msg: Arguments);

/// A [`LogFn`] that does nothing.
pub fn no_op(_: Level, _: Arguments) {}

/// The global log function.
static GLOBAL_LOG_FN: AtomicPtr<()> = AtomicPtr::new(no_op as *mut ());

/// Sets the global log function.
#[inline(always)]
pub fn set_global_log_fn(log_fn: LogFn) {
    GLOBAL_LOG_FN.store(log_fn as *mut (), Relaxed);
}

/// Returns the global log function.
#[inline(always)]
pub fn get_global_log_fn() -> LogFn {
    // SAFETY:
    //  The global log fn always points to a function pointer of type `LogFn`.
    unsafe { core::mem::transmute(GLOBAL_LOG_FN.load(Relaxed)) }
}

/// Logs a message with the [`Level::Trace`] log level.
pub macro trace {
    ($($arg:tt)*) => {
        $crate::log::get_global_log_fn()($crate::log::Level::Trace, format_args!($($arg)*))
    }
}

/// Logs a message with the [`Level::Info`] log level.
pub macro info {
    ($($arg:tt)*) => {
        $crate::log::get_global_log_fn()($crate::log::Level::Info, format_args!($($arg)*))
    }
}

/// Logs a message with the [`Level::Warn`] log level.
pub macro warn {
    ($($arg:tt)*) => {
        $crate::log::get_global_log_fn()($crate::log::Level::Warn, format_args!($($arg)*))
    }
}

/// Logs a message with the [`Level::Error`] log level.
pub macro error {
    ($($arg:tt)*) => {
        $crate::log::get_global_log_fn()($crate::log::Level::Error, format_args!($($arg)*))
    }
}
