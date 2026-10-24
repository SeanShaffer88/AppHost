//! Platform paths for the generated app.
//!
//! On Windows the data root is `%APPDATA%\<APP_ID>`, resolved from
//! the Known Folder API rather than by trusting an environment
//! variable. The `APPHOST_DATA_DIR` environment variable is an exact
//! directory override intended for diagnostics and automated tests;
//! the library's `AppContext::start_at` is the normal test seam.

use std::path::PathBuf;

/// Resolve this app's data directory.
pub fn app_data_dir(app_id: &str) -> Result<PathBuf, String> {
    if let Some(value) = std::env::var_os("APPHOST_DATA_DIR") {
        if !value.is_empty() {
            return Ok(PathBuf::from(value));
        }
    }

    #[cfg(windows)]
    {
        roaming_app_data().map(|base| base.join(app_id))
    }

    #[cfg(not(windows))]
    {
        // Development/self-check fallback only. The generated product
        // targets Windows; this branch lets the wiring be exercised
        // by host-side tests on other platforms.
        if let Some(value) = std::env::var_os("APPDATA") {
            if !value.is_empty() {
                return Ok(PathBuf::from(value).join(app_id));
            }
        }
        if let Some(value) = std::env::var_os("XDG_CONFIG_HOME") {
            if !value.is_empty() {
                return Ok(PathBuf::from(value).join(app_id));
            }
        }
        if let Some(value) = std::env::var_os("HOME") {
            if !value.is_empty() {
                return Ok(PathBuf::from(value).join(".config").join(app_id));
            }
        }
        Ok(std::env::temp_dir().join(app_id))
    }
}

#[cfg(windows)]
fn roaming_app_data() -> Result<PathBuf, String> {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    // FOLDERID_RoamingAppData.
    const FOLDERID_ROAMING_APP_DATA: Guid = Guid {
        data1: 0x3EB6_85DB,
        data2: 0x65F9,
        data3: 0x4CF6,
        data4: [0xA0, 0x3A, 0xE3, 0xEF, 0x65, 0x72, 0x9F, 0x3D],
    };

    #[link(name = "shell32")]
    extern "system" {
        fn SHGetKnownFolderPath(
            rfid: *const Guid,
            flags: u32,
            token: *mut c_void,
            path: *mut *mut u16,
        ) -> i32;
    }

    #[link(name = "ole32")]
    extern "system" {
        fn CoTaskMemFree(pv: *mut c_void);
    }

    unsafe {
        let mut ptr: *mut u16 = std::ptr::null_mut();
        let hr = SHGetKnownFolderPath(
            &FOLDERID_ROAMING_APP_DATA,
            0,
            std::ptr::null_mut(),
            &mut ptr,
        );
        if hr != 0 || ptr.is_null() {
            return Err(format!(
                "SHGetKnownFolderPath(FOLDERID_RoamingAppData) failed: HRESULT 0x{hr:08X}"
            ));
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
        CoTaskMemFree(ptr as *mut c_void);
        Ok(PathBuf::from(text))
    }
}

/// Escape a string for inclusion inside a JSON string literal.
pub fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 8);
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
