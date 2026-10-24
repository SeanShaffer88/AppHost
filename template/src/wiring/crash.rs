//! Crash wiring — CrashKit (https://github.com/SeanShaffer88/CrashKit).
//!
//! Exact API used: the kit's public Rust FFI module
//! (`crashkit::ffi::{ck_install, ck_is_installed, ck_set_context}`),
//! backed by its `crashkit` rlib from the pinned git dependency.
//! CrashKit owns the Windows unhandled-exception filter, minidump,
//! and JSON sidecar. Its handler is Windows-only: off Windows,
//! `ck_install` returns CK_ERR_NOT_SUPPORTED, which this module
//! surfaces as a status instead of failing application startup.

use crashkit::ffi;
use std::ffi::{CStr, CString};
use std::path::Path;

/// Result of attempting to install CrashKit at startup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrashStatus {
    Installed,
    Unsupported(String),
    Failed { code: i32, message: String },
}

impl CrashStatus {
    pub fn summary(&self) -> String {
        match self {
            Self::Installed => "installed".to_string(),
            Self::Unsupported(message) => format!("unsupported: {message}"),
            Self::Failed { code, message } => format!("failed ({code}): {message}"),
        }
    }
}

/// Install CrashKit for dumps under `crashes_dir`.
pub fn install(crashes_dir: &Path, app_name: &str) -> CrashStatus {
    if let Err(e) = std::fs::create_dir_all(crashes_dir) {
        return CrashStatus::Failed {
            code: -1,
            message: format!("cannot create crash directory: {e}"),
        };
    }
    let dir_c = match CString::new(crashes_dir.to_string_lossy().into_owned()) {
        Ok(v) => v,
        Err(_) => {
            return CrashStatus::Failed {
                code: -1,
                message: "crash directory contains a NUL byte".to_string(),
            }
        }
    };
    let app_c = match CString::new(app_name) {
        Ok(v) => v,
        Err(_) => {
            return CrashStatus::Failed {
                code: -1,
                message: "app name contains a NUL byte".to_string(),
            }
        }
    };

    // Preset 1 = CrashKit "standard" dump.
    let rc = ffi::ck_install(dir_c.as_ptr(), app_c.as_ptr(), 1);
    if rc == crashkit::CK_OK {
        // Context is diagnostic metadata included in every sidecar.
        if let (Ok(key), Ok(value)) = (CString::new("app"), CString::new(app_name)) {
            let _ = ffi::ck_set_context(key.as_ptr(), value.as_ptr());
        }
        CrashStatus::Installed
    } else if rc == crashkit::ERR_NOT_SUPPORTED {
        CrashStatus::Unsupported(last_error())
    } else {
        CrashStatus::Failed {
            code: rc,
            message: last_error(),
        }
    }
}

/// Whether CrashKit's process-global handler is installed.
pub fn is_installed() -> bool {
    let installed = ffi::ck_is_installed();
    installed == 1
}

fn last_error() -> String {
    let ptr = ffi::ck_last_error();
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}
