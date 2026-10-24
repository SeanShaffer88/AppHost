//! Settings wiring — SettingsKit (https://github.com/SeanShaffer88/SettingsKit).
//!
//! Exact API used: the kit's public Rust FFI module
//! (`settings_kit::ffi::{sk_open, sk_get_i64, sk_get_string,
//! sk_set_i64, sk_set_string, sk_save, sk_close}`), backed by its
//! `settings_kit` rlib from the pinned git dependency. SettingsKit
//! owns the JSON document, schema key, defaults-on-read behaviour,
//! and atomic save; this module only maps the app's fields.

use settings_kit::ffi::{self, SkDoc};
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};

const SCHEMA: i64 = 1;

/// The app's persisted settings.
#[derive(Clone, Debug, PartialEq)]
pub struct AppSettings {
    pub window_x: i32,
    pub window_y: i32,
    pub launch_count: i64,
    pub update_manifest_url: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            window_x: 120,
            window_y: 120,
            launch_count: 0,
            update_manifest_url: String::new(),
        }
    }
}

/// A live SettingsKit document handle.
pub struct SettingsDocument {
    handle: *mut SkDoc,
    path: PathBuf,
}

impl SettingsDocument {
    pub fn open(path: &Path) -> Result<Self, String> {
        let path_c = c_string(path.to_string_lossy().as_ref())?;
        let mut handle: *mut SkDoc = std::ptr::null_mut();
        let rc = ffi::sk_open(path_c.as_ptr(), SCHEMA, 0, &mut handle);
        if rc != settings_kit::SK_OK || handle.is_null() {
            return Err(format!(
                "SettingsKit sk_open failed ({rc}): {}",
                last_error()
            ));
        }
        Ok(Self {
            handle,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn snapshot(&self) -> Result<AppSettings, String> {
        let defaults = AppSettings::default();
        Ok(AppSettings {
            window_x: self.get_i64("window.x", defaults.window_x as i64)? as i32,
            window_y: self.get_i64("window.y", defaults.window_y as i64)? as i32,
            launch_count: self.get_i64("launch_count", defaults.launch_count)?,
            update_manifest_url: self
                .get_string("updates.manifest_url", &defaults.update_manifest_url)?,
        })
    }

    pub fn save_snapshot(&mut self, settings: &AppSettings) -> Result<(), String> {
        self.set_i64("window.x", settings.window_x as i64)?;
        self.set_i64("window.y", settings.window_y as i64)?;
        self.set_i64("launch_count", settings.launch_count)?;
        self.set_string("updates.manifest_url", &settings.update_manifest_url)?;
        self.save()
    }

    pub fn set_window_position(&mut self, x: i32, y: i32) -> Result<(), String> {
        self.set_i64("window.x", x as i64)?;
        self.set_i64("window.y", y as i64)?;
        self.save()
    }

    fn get_i64(&self, path: &str, default: i64) -> Result<i64, String> {
        let path_c = c_string(path)?;
        let mut out = default;
        let mut found = 0;
        let rc = ffi::sk_get_i64(self.handle, path_c.as_ptr(), default, &mut out, &mut found);
        if rc != settings_kit::SK_OK {
            return Err(format!(
                "SettingsKit sk_get_i64({path}) failed ({rc}): {}",
                last_error()
            ));
        }
        Ok(out)
    }

    fn get_string(&self, path: &str, default: &str) -> Result<String, String> {
        let path_c = c_string(path)?;
        let default_c = c_string(default)?;
        let mut found = 0;
        let ptr = ffi::sk_get_string(self.handle, path_c.as_ptr(), default_c.as_ptr(), &mut found);
        if found < 0 {
            return Err(format!(
                "SettingsKit sk_get_string({path}) failed: {}",
                last_error()
            ));
        }
        if ptr.is_null() {
            return Ok(default.to_string());
        }
        let value = unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        ffi::sk_free_string(ptr);
        Ok(value)
    }

    fn set_i64(&mut self, path: &str, value: i64) -> Result<(), String> {
        let path_c = c_string(path)?;
        let rc = ffi::sk_set_i64(self.handle, path_c.as_ptr(), value);
        check(rc, "sk_set_i64")
    }

    fn set_string(&mut self, path: &str, value: &str) -> Result<(), String> {
        let path_c = c_string(path)?;
        let value_c = c_string(value)?;
        let rc = ffi::sk_set_string(self.handle, path_c.as_ptr(), value_c.as_ptr());
        check(rc, "sk_set_string")
    }

    fn save(&mut self) -> Result<(), String> {
        let rc = ffi::sk_save(self.handle);
        check(rc, "sk_save")
    }
}

impl Drop for SettingsDocument {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            ffi::sk_close(self.handle);
            self.handle = std::ptr::null_mut();
        }
    }
}

fn check(rc: i32, operation: &str) -> Result<(), String> {
    if rc == settings_kit::SK_OK {
        Ok(())
    } else {
        Err(format!(
            "SettingsKit {operation} failed ({rc}): {}",
            last_error()
        ))
    }
}

fn c_string(value: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| "value contains a NUL byte".to_string())
}

fn last_error() -> String {
    let ptr = ffi::sk_last_error();
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}
