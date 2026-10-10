//! Logging configuration for libkrun.

use krun_sys as sys;

#[repr(u32)]
pub enum LogLevel {
    Off = sys::KRUN_LOG_LEVEL_OFF,
    Error = sys::KRUN_LOG_LEVEL_ERROR,
    Warn = sys::KRUN_LOG_LEVEL_WARN,
    Info = sys::KRUN_LOG_LEVEL_INFO,
    Debug = sys::KRUN_LOG_LEVEL_DEBUG,
    Trace = sys::KRUN_LOG_LEVEL_TRACE,
}

/// Sets the log level for the krun library.
/// See [`LogLevel`].
pub fn set_log_level(level: LogLevel) -> Option<()> {
    let res = unsafe { sys::krun_set_log_level(level as _) };
    if res >= 0 { Some(()) } else { None }
}
