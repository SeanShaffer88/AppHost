//! Logging wiring — LogKit (https://github.com/SeanShaffer88/LogKit).
//!
//! Exact API used: the kit's public Rust FFI module
//! (`log_kit::ffi::{lk_open, lk_log, lk_log_fields, lk_flush,
//! lk_close}`), backed by its `log_kit` rlib from the pinned git
//! dependency. LogKit owns the frozen house line format and the
//! rolling file sink; this module is only a typed Rust wrapper.

use log_kit::ffi::{self, LkLogger};
use std::ffi::{CStr, CString};
use std::path::Path;

pub const LEVEL_INFO: i32 = log_kit::LEVEL_INFO;
pub const LEVEL_WARN: i32 = log_kit::LEVEL_WARN;
pub const LEVEL_ERROR: i32 = log_kit::LEVEL_ERROR;

/// A live LogKit logger handle.
pub struct Logger {
    handle: *mut LkLogger,
}

impl Logger {
    pub fn open(path: &Path) -> Result<Self, String> {
        let path_c = c_string(path.to_string_lossy().as_ref())?;
        let mut handle: *mut LkLogger = std::ptr::null_mut();
        // 1 MiB per file, current + 3 rolled files. Flush-per-record
        // is LogKit's default crash-safety behaviour.
        let rc = ffi::lk_open(path_c.as_ptr(), LEVEL_INFO, 1_048_576, 3, 0, &mut handle);
        if rc != log_kit::LK_OK || handle.is_null() {
            return Err(format!("LogKit lk_open failed ({rc}): {}", last_error()));
        }
        Ok(Self { handle })
    }

    pub fn info(&self, target: &str, message: &str) -> Result<(), String> {
        self.log(LEVEL_INFO, target, message)
    }

    pub fn warn(&self, target: &str, message: &str) -> Result<(), String> {
        self.log(LEVEL_WARN, target, message)
    }

    pub fn error(&self, target: &str, message: &str) -> Result<(), String> {
        self.log(LEVEL_ERROR, target, message)
    }

    pub fn log(&self, level: i32, target: &str, message: &str) -> Result<(), String> {
        let target_c = c_string(target)?;
        let message_c = c_string(message)?;
        let rc = ffi::lk_log(self.handle, level, target_c.as_ptr(), message_c.as_ptr());
        check(rc, "lk_log")
    }

    pub fn log_fields(
        &self,
        level: i32,
        target: &str,
        message: &str,
        fields_json: &str,
    ) -> Result<(), String> {
        let target_c = c_string(target)?;
        let message_c = c_string(message)?;
        let fields_c = c_string(fields_json)?;
        let rc = ffi::lk_log_fields(
            self.handle,
            level,
            target_c.as_ptr(),
            message_c.as_ptr(),
            fields_c.as_ptr(),
        );
        check(rc, "lk_log_fields")
    }

    pub fn flush(&self) -> Result<(), String> {
        let rc = ffi::lk_flush(self.handle);
        check(rc, "lk_flush")
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            ffi::lk_close(self.handle);
            self.handle = std::ptr::null_mut();
        }
    }
}

fn check(rc: i32, operation: &str) -> Result<(), String> {
    if rc == log_kit::LK_OK {
        Ok(())
    } else {
        Err(format!(
            "LogKit {operation} failed ({rc}): {}",
            last_error()
        ))
    }
}

fn c_string(value: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| "value contains a NUL byte".to_string())
}

fn last_error() -> String {
    let ptr = ffi::lk_last_error();
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}
