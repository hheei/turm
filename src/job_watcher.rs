use std::{path::Path, process::Command, thread, time::Duration};

use crossbeam::channel::Sender;

use crate::app::{AppMessage, Job};

mod parser;

const SACCT_FIELDS: [&str; 16] = [
    "JobID",
    "JobIDRaw",
    "JobName",
    "State",
    "User",
    "Elapsed",
    "Timelimit",
    "Start",
    "AllocTRES",
    "Partition",
    "NodeList",
    "Stdout",
    "Stderr",
    "Reason",
    "WorkDir",
    "Command",
];

struct JobWatcher {
    app: Sender<AppMessage>,
    interval: Duration,
    sacct_args: Vec<String>,
}

pub struct JobWatcherHandle {}

impl JobWatcher {
    fn new(app: Sender<AppMessage>, interval: Duration, sacct_args: Vec<String>) -> Self {
        Self {
            app,
            interval,
            sacct_args,
        }
    }

    fn run(self) {
        loop {
            if let Ok(jobs) = fetch_jobs_with(Path::new("sacct"), &self.sacct_args)
                && self.app.send(AppMessage::Jobs(jobs)).is_err()
            {
                return;
            }
            thread::sleep(self.interval);
        }
    }
}

pub(crate) fn fetch_jobs_with(sacct: &Path, sacct_args: &[String]) -> Result<Vec<Job>, String> {
    let output = Command::new(sacct)
        .args(sacct_args)
        .arg("--allocations")
        .arg("--starttime=now-2days")
        .arg("--endtime=now")
        .arg("--noheader")
        .arg("--parsable2")
        .arg(format!("--format={}", SACCT_FIELDS.join(",")))
        .output()
        .map_err(|error| format!("failed to execute {}: {error}", sacct.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("{} exited with {}", sacct.display(), output.status)
        } else {
            stderr
        });
    }

    let jobs = parser::parse_sacct_jobs(&String::from_utf8_lossy(&output.stdout));
    if sacct_args.iter().any(|arg| arg == "--allusers")
        && let Ok(user) = std::env::var("USER")
    {
        Ok(filter_group_jobs(jobs, &user))
    } else {
        Ok(jobs)
    }
}

pub(crate) fn filter_group_jobs(jobs: Vec<Job>, user: &str) -> Vec<Job> {
    jobs.into_iter()
        .filter(|job| !is_historical(&job.state) || job.user == user)
        .collect()
}

fn is_historical(state: &str) -> bool {
    matches!(
        state.split_whitespace().next().unwrap_or_default(),
        "BOOT_FAIL"
            | "CANCELLED"
            | "COMPLETED"
            | "DEADLINE"
            | "FAILED"
            | "NODE_FAIL"
            | "OUT_OF_MEMORY"
            | "PREEMPTED"
            | "REVOKED"
            | "SPECIAL_EXIT"
            | "TIMEOUT"
    )
}

pub(crate) fn parse_sacct_jobs(output: &str) -> Vec<Job> {
    parser::parse_sacct_jobs(output)
}

impl JobWatcherHandle {
    pub fn new(app: Sender<AppMessage>, interval: Duration, sacct_args: Vec<String>) -> Self {
        let actor = JobWatcher::new(app, interval, sacct_args);
        thread::spawn(move || actor.run());

        Self {}
    }
}
