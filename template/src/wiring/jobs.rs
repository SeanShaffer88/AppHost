//! Background-job wiring — JobKit (https://github.com/SeanShaffer88/JobKit).
//!
//! Exact API used: JobKit's native Rust surface
//! (`job_kit::{parse_config, Job, Status}`), backed by its `job_kit`
//! rlib from the pinned git dependency. The demo job has three
//! weighted steps (25 + 35 + 40 = 100 units). Each step reports
//! progress and calls `Job::checkpoint`, the cooperative pause/cancel
//! point, between short work chunks; the whole job takes about two
//! seconds when not paused.

use job_kit::{Job, Status};
use std::ffi::c_void;
use std::sync::Arc;
use std::time::Duration;

struct StepSpec {
    chunks: u64,
    chunk_delay_ms: u64,
    weight: u64,
}

/// An owned, running-or-finished demo job.
///
/// The boxed step specs must outlive JobKit's stored callback user
/// pointers, so they live here beside the job. Dropping a DemoJob
/// shuts the job down (JobKit cancels and joins its worker).
pub struct DemoJob {
    job: Arc<Job>,
    // Boxed for pointer stability: JobKit stores each step's
    // user_data pointer while later steps are still being added, so
    // a spec must never move when the Vec grows.
    #[allow(clippy::vec_box)]
    _step_specs: Vec<Box<StepSpec>>,
}

impl DemoJob {
    pub fn status(&self) -> Status {
        self.job.status()
    }

    /// Block until terminal or `timeout_ms` elapses; true = terminal.
    pub fn wait(&self, timeout_ms: u32) -> bool {
        self.job.wait(timeout_ms)
    }

    pub fn shutdown(&mut self) {
        self.job.shutdown();
    }
}

impl Drop for DemoJob {
    fn drop(&mut self) {
        self.job.shutdown();
    }
}

/// Create (but do not start) the demo job.
pub fn create_demo_job() -> Result<DemoJob, String> {
    let config = job_kit::parse_config("{\"name\":\"AppHost demo job\",\"total_units\":100}")
        .map_err(|(code, message)| format!("JobKit config failed ({code}): {message}"))?;
    let job = Job::create(config);
    let specs = vec![
        StepSpec {
            chunks: 5,
            chunk_delay_ms: 100,
            weight: 25,
        },
        StepSpec {
            chunks: 7,
            chunk_delay_ms: 100,
            weight: 35,
        },
        StepSpec {
            chunks: 8,
            chunk_delay_ms: 100,
            weight: 40,
        },
    ];
    let names = ["Prepare", "Work", "Finish"];
    let mut demo = DemoJob {
        job,
        _step_specs: Vec::new(),
    };
    for (spec, name) in specs.into_iter().zip(names) {
        let boxed = Box::new(spec);
        let user_data = &*boxed as *const StepSpec as *mut c_void;
        demo._step_specs.push(boxed);
        let weight = demo._step_specs.last().unwrap().weight;
        demo.job
            .add_step(name, weight, demo_step, user_data)
            .map_err(|(code, message)| format!("JobKit add_step failed ({code}): {message}"))?;
    }
    Ok(demo)
}

/// Create, start, and return the demo job for GUI polling.
pub fn start_demo_job() -> Result<DemoJob, String> {
    let demo = create_demo_job()?;
    demo.job
        .start()
        .map_err(|(code, message)| format!("JobKit start failed ({code}): {message}"))?;
    Ok(demo)
}

/// Create and run the demo job to a terminal state.
pub fn run_demo_job() -> Result<Status, String> {
    let mut demo = start_demo_job()?;
    if !demo.wait(30_000) {
        demo.shutdown();
        return Err("demo job did not reach a terminal state within 30 seconds".to_string());
    }
    let status = demo.status();
    demo.shutdown();
    if status.state == job_kit::State::Failed {
        return Err(format!(
            "demo job failed: {}",
            status.error.as_deref().unwrap_or("unknown error")
        ));
    }
    if status.state != job_kit::State::Completed {
        return Err(format!(
            "demo job ended in state {} instead of completed",
            status.state.as_str()
        ));
    }
    Ok(status)
}

extern "C" fn demo_step(user_data: *mut c_void, job: *mut Job) -> i32 {
    if user_data.is_null() || job.is_null() {
        return job_kit::ERR_INVALID_ARG;
    }
    let spec = unsafe { &*(user_data as *const StepSpec) };
    let job = unsafe { &*job };
    for chunk in 1..=spec.chunks {
        let checkpoint = job.checkpoint();
        if checkpoint != job_kit::JK_OK {
            return checkpoint;
        }
        std::thread::sleep(Duration::from_millis(spec.chunk_delay_ms));
        job.report(spec.weight.saturating_mul(chunk) / spec.chunks);
    }
    job.checkpoint()
}

/// Human-readable one-line status for the GUI status bar.
pub fn format_status(status: &Status) -> String {
    let eta = match status.eta_ms {
        Some(ms) => format!("{} ms", ms),
        None => "unknown".to_string(),
    };
    let step = status.step_name.as_deref().unwrap_or("—");
    format!(
        "Job {} — {:.0}% (step {}/{}: {}), ETA {}",
        status.state.as_str(),
        status.percent,
        status.step_index + 1,
        status.step_count,
        step,
        eta
    )
}
