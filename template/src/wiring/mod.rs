//! The generated app's wiring layer.
//!
//! Each submodule is a thin, honest adapter to one shipped kit; the
//! kit—not this template—owns the hard behaviour. `AppContext` is the
//! composition root used by both the Windows GUI and the headless
//! self-check.

pub mod crash;
pub mod jobs;
pub mod logging;
pub mod proc;
pub mod settings;
pub mod updates;

pub use crash::CrashStatus;
pub use jobs::DemoJob;
pub use proc::CHILD_MARKER;
pub use settings::AppSettings;
pub use updates::UpdateCheck;

use crate::platform;
use crate::{APP_ID, APP_NAME, VERSION};
use std::path::{Path, PathBuf};

/// Everything the app needs after startup wiring has run.
pub struct AppContext {
    data_dir: PathBuf,
    settings: AppSettings,
    settings_doc: settings::SettingsDocument,
    logger: logging::Logger,
    crash_status: CrashStatus,
    already_running: bool,
}

impl AppContext {
    /// Start with the platform-resolved data directory.
    pub fn start(already_running: bool) -> Result<Self, String> {
        let root = platform::app_data_dir(APP_ID)?;
        Self::start_at(&root, already_running)
    }

    /// Start with an explicit data directory (the test seam).
    pub fn start_at(data_dir: &Path, already_running: bool) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir)
            .map_err(|e| format!("cannot create app data directory: {e}"))?;

        let settings_path = data_dir.join("settings.json");
        let mut settings_doc = settings::SettingsDocument::open(&settings_path)?;
        let mut settings = settings_doc.snapshot()?;
        settings.launch_count = settings.launch_count.saturating_add(1);
        settings_doc.save_snapshot(&settings)?;

        let crash_status = crash::install(&data_dir.join("crashes"), APP_NAME);
        let logger = logging::Logger::open(&data_dir.join("app.log"))?;

        let context = Self {
            data_dir: data_dir.to_path_buf(),
            settings,
            settings_doc,
            logger,
            crash_status,
            already_running,
        };
        context.log_startup()?;
        if !matches!(context.crash_status, CrashStatus::Installed) {
            let _ = context.logger.warn(
                "startup",
                &format!("crash capture {}", context.crash_status.summary()),
            );
        }
        Ok(context)
    }

    fn log_startup(&self) -> Result<(), String> {
        // Concatenation, not a format literal — see proc.rs for why
        // this file avoids doubled-brace sequences.
        let mut fields = String::from("{");
        fields.push_str(&format!(
            "\"app\":\"{}\",\"version\":\"{}\",\"already_running\":{},\"settings_path\":\"{}\"",
            platform::json_escape(APP_NAME),
            platform::json_escape(VERSION),
            self.already_running,
            platform::json_escape(&self.settings_doc.path().to_string_lossy()),
        ));
        fields.push('}');
        self.logger.log_fields(
            logging::LEVEL_INFO,
            "startup",
            "application started",
            &fields,
        )
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn settings(&self) -> &AppSettings {
        &self.settings
    }

    pub fn settings_path(&self) -> &Path {
        self.settings_doc.path()
    }

    pub fn crash_status(&self) -> &CrashStatus {
        &self.crash_status
    }

    pub fn already_running(&self) -> bool {
        self.already_running
    }

    pub fn logger(&self) -> &logging::Logger {
        &self.logger
    }

    /// Persist a new main-window position.
    pub fn save_window_position(&mut self, x: i32, y: i32) -> Result<(), String> {
        self.settings.window_x = x;
        self.settings.window_y = y;
        self.settings_doc.set_window_position(x, y)
    }

    /// Start the JobKit demo job for GUI polling.
    pub fn start_demo_job(&self) -> Result<DemoJob, String> {
        self.logger.info("jobs", "demo job requested")?;
        jobs::start_demo_job()
    }

    /// Run the JobKit demo job to completion and log its outcome.
    pub fn run_demo_job(&self) -> Result<job_kit::Status, String> {
        self.logger.info("jobs", "demo job requested")?;
        let status = jobs::run_demo_job()?;
        self.logger.info(
            "jobs",
            &format!(
                "demo job {} percent={} elapsed_ms={}",
                status.state.as_str(),
                status.percent,
                status.elapsed_ms
            ),
        )?;
        Ok(status)
    }

    /// Run the ProcessKit demo child and log its captured output.
    pub fn run_demo_child(&self) -> Result<process_kit::ProcResult, String> {
        self.logger.info("proc", "demo child requested")?;
        let result = proc::run_demo_child()?;
        self.logger.info(
            "proc",
            &format!(
                "demo child state={} exit_code={:?} stdout={}",
                result.state.name(),
                result.exit_code,
                result.stdout.trim()
            ),
        )?;
        Ok(result)
    }

    /// Run the UpdateKit check and log its outcome. An empty
    /// configured URL returns (and logs) the disabled outcome before
    /// UpdateKit is called.
    pub fn check_for_updates(&self) -> UpdateCheck {
        let outcome = updates::check_for_updates(&self.settings.update_manifest_url);
        let _ = self.logger.info("updates", &outcome.summary());
        outcome
    }
}
