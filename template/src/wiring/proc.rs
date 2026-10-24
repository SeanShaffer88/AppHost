//! Child-process wiring — ProcessKit
//! (https://github.com/SeanShaffer88/ProcessKit).
//!
//! Exact API used: ProcessKit's native Rust surface
//! (`process_kit::{parse_config, run_config, ProcResult}`), backed by
//! its `process_kit` rlib from the pinned git dependency. ProcessKit
//! owns spawning, capture, timeout policy, and (on Windows) the Job
//! Object tree-kill guarantee.

use crate::platform::json_escape;
use process_kit::ProcResult;

/// Marker printed by the demo child and asserted by the tests.
pub const CHILD_MARKER: &str = "APPHOST_DEMO_CHILD_OK";

/// Run the short platform echo command and return its captured result.
pub fn run_demo_child() -> Result<ProcResult, String> {
    #[cfg(windows)]
    let (exe, args) = ("cmd.exe", vec!["/C", "echo", CHILD_MARKER]);
    #[cfg(not(windows))]
    let (exe, args) = ("/bin/echo", vec![CHILD_MARKER]);

    let args_json = args
        .iter()
        .map(|arg| format!("\"{}\"", json_escape(arg)))
        .collect::<Vec<_>>()
        .join(",");
    // Built by concatenation so this source file contains no
    // doubled-brace sequences (the scaffold's placeholder scan would
    // flag them in a generated tree).
    let mut config_json = String::from("{");
    config_json.push_str(&format!(
        "\"exe\":\"{}\",\"args\":[{}],\"capture\":true,\"timeout_ms\":10000,\"kill_tree\":true,\"hidden\":true",
        json_escape(exe),
        args_json
    ));
    config_json.push('}');
    let config = process_kit::parse_config(&config_json)
        .map_err(|(code, message)| format!("ProcessKit config failed ({code}): {message}"))?;
    let (result, spawn_error) = process_kit::run_config(config);
    if let Some((code, message)) = spawn_error {
        return Err(format!("ProcessKit demo child failed ({code}): {message}"));
    }
    Ok(result)
}

/// Whether a result represents the expected successful echo.
pub fn is_successful_demo(result: &ProcResult) -> bool {
    result.state == process_kit::ProcState::Exited
        && result.exit_code == Some(0)
        && result.stdout.contains(CHILD_MARKER)
}
