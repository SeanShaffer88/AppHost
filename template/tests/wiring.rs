//! Wiring integration tests.
//!
//! These exercise the REAL shipped kits through the generated app's
//! library: no kit is mocked. They run on any host (the GUI binary is
//! stubbed off Windows; the wiring library is not).

use app_skeleton::wiring::crash;
use app_skeleton::{AppContext, UpdateCheck, CHILD_MARKER};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "apphost-{}-{}-{}",
        label,
        std::process::id(),
        stamp
    ))
}

#[test]
fn settings_launch_counter_persists_through_settingskit() {
    let root = temp_root("settings");
    {
        let context = AppContext::start_at(&root, false).unwrap();
        assert_eq!(context.settings().launch_count, 1);
        assert_eq!(context.settings().window_x, 120);
    }
    {
        let context = AppContext::start_at(&root, true).unwrap();
        assert_eq!(context.settings().launch_count, 2);
        assert!(context.already_running());
    }
    let raw = std::fs::read_to_string(root.join("settings.json")).unwrap();
    assert!(raw.contains("\"launch_count\": 2"), "settings file: {raw}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn startup_log_matches_logkit_house_format() {
    let root = temp_root("logging");
    {
        let context = AppContext::start_at(&root, false).unwrap();
        context.logger().flush().unwrap();
        assert!(context.data_dir().join("app.log").exists());
    }
    let log = std::fs::read_to_string(root.join("app.log")).unwrap();
    let startup = log
        .lines()
        .find(|line| line.contains("startup: application started"))
        .expect("startup line present");
    // LogKit house format: `<ISO-8601 UTC ms> <LEVEL> <target>: <msg> k=v...`
    let mut parts = startup.splitn(4, ' ');
    let timestamp = parts.next().unwrap();
    assert_eq!(timestamp.len(), 24, "timestamp: {timestamp}");
    assert!(timestamp.ends_with('Z'));
    assert_eq!(timestamp.as_bytes()[4], b'-');
    assert_eq!(timestamp.as_bytes()[10], b'T');
    assert_eq!(parts.next().unwrap(), "INFO");
    // LogKit renders field values bare when they need no quoting.
    assert!(startup.contains(&format!("app={}", app_skeleton::APP_NAME)));
    assert!(startup.contains("already_running=false"));
    assert!(startup.contains("settings_path="));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn demo_job_completes_through_jobkit_at_100_percent() {
    let root = temp_root("job");
    let context = AppContext::start_at(&root, false).unwrap();
    let status = context.run_demo_job().unwrap();
    assert_eq!(status.state.as_str(), "completed");
    assert_eq!(status.percent, 100.0);
    assert_eq!(status.step_count, 3);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn demo_child_captures_marker_through_processkit() {
    let root = temp_root("proc");
    let context = AppContext::start_at(&root, false).unwrap();
    let result = context.run_demo_child().unwrap();
    assert_eq!(result.exit_code, Some(0));
    assert!(
        result.stdout.contains(CHILD_MARKER),
        "stdout: {}",
        result.stdout
    );
    assert!(app_skeleton::wiring::proc::is_successful_demo(&result));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn crash_wiring_installs_or_reports_unsupported() {
    let root = temp_root("crash");
    let context = AppContext::start_at(&root, false).unwrap();
    // CrashKit's handler is Windows-only by design: on Windows the
    // install must have succeeded and ck_is_installed must read 1.
    // Off Windows ck_install returns CK_ERR_NOT_SUPPORTED and the
    // wiring surfaces that as CrashStatus::Unsupported instead of
    // failing startup.
    #[cfg(windows)]
    {
        assert_eq!(
            *context.crash_status(),
            app_skeleton::CrashStatus::Installed
        );
        assert!(crash::is_installed());
    }
    #[cfg(not(windows))]
    {
        assert!(matches!(
            context.crash_status(),
            app_skeleton::CrashStatus::Unsupported(_)
        ));
        assert!(!crash::is_installed());
    }
    assert!(root.join("crashes").is_dir());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn update_check_is_disabled_without_a_manifest_url() {
    let root = temp_root("updates");
    let context = AppContext::start_at(&root, false).unwrap();
    assert_eq!(context.settings().update_manifest_url, "");
    let outcome = context.check_for_updates();
    assert_eq!(outcome, UpdateCheck::Disabled);
    assert_eq!(
        outcome.summary(),
        "update check disabled — no manifest URL configured"
    );
    let _ = std::fs::remove_dir_all(&root);
}
