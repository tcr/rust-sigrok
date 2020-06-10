//! Logging
//!
//! Control the Sigrok message logging functionality.

pub use crate::enums::LogLevel;
use sigrok_sys::{sr_log_loglevel_get, sr_log_loglevel_set};
use std::convert::TryInto;

/// Set the current log level.
pub fn set_log_level(level: LogLevel) {
    unsafe {
        sr_log_loglevel_set(level.into());
    }
}

/// Retrieve the current log level.
pub fn get_log_level() -> LogLevel {
    // Anything invalid is probably a future, higher log level, so let's say it's the highest we
    // know about
    unsafe { sr_log_loglevel_get().try_into().unwrap_or(LogLevel::Spew) }
}
