use super::SlurmClient;
use crate::app::Job;
use std::{path::Path, process::Command};

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

/// Accounting schemas vary even within a Slurm release (vendor extensions).
/// Keep optional metadata out of requests when the installed CLI lacks it.
pub(super) struct HistoryFormat {
    columns: Vec<usize>,
}

impl HistoryFormat {
    pub(super) fn detect(client: &SlurmClient) -> Result<Self, String> {
        let help = client
            .output(Command::new(&client.sacct).arg("--helpformat"))
            .map_err(|error| error.to_string())?;
        let mut columns = Vec::new();
        for (column, field) in SACCT_FIELDS.iter().enumerate() {
            if help
                .split_whitespace()
                .any(|name| name.eq_ignore_ascii_case(field))
            {
                columns.push(column);
            } else if column < 11 {
                return Err(format!("sacct does not advertise required field {field}"));
            }
        }
        Ok(Self { columns })
    }

    fn argument(&self) -> String {
        self.columns
            .iter()
            .map(|&column| SACCT_FIELDS[column])
            .collect::<Vec<_>>()
            .join(",")
    }
}

pub(crate) fn fetch_jobs_with(sacct: &Path, sacct_args: &[String]) -> Result<Vec<Job>, String> {
    let mut client = SlurmClient::in_directory("");
    client.sacct = sacct.into();
    fetch_history(&client, sacct_args)
}

pub(crate) fn fetch_history(
    client: &SlurmClient,
    sacct_args: &[String],
) -> Result<Vec<Job>, String> {
    let format = client.history_format()?;
    let output = client
        .output(
            Command::new(&client.sacct)
                .args(sacct_args)
                .arg("--allocations")
                .arg("--starttime=now-2days")
                .arg("--endtime=now")
                .arg("--noheader")
                .arg("--parsable2")
                .arg(format!("--format={}", format.argument())),
        )
        .map_err(|error| error.to_string())?;

    let all_users_user = sacct_args
        .iter()
        .any(|arg| arg == "--allusers")
        .then(|| std::env::var("USER").ok())
        .flatten();
    let jobs =
        parser::parse_sacct_jobs_with_columns(&output, all_users_user.as_deref(), &format.columns);
    if let Some(user) = all_users_user {
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

pub(crate) fn is_historical(state: &str) -> bool {
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
            | "TIMEOUT"
            | "BF"
            | "CA"
            | "CD"
            | "DL"
            | "F"
            | "NF"
            | "OOM"
            | "PR"
            | "RV"
            | "TO"
    )
}

pub(crate) fn parse_sacct_jobs(output: &str) -> Vec<Job> {
    parser::parse_sacct_jobs(output)
}

// Long format fields avoid version-dependent JSON schemas. A zero minimum
// width preserves complete names and paths.
const SQUEUE_FIELDS: [&str; 17] = [
    "JobID",
    "ArrayJobID",
    "ArrayTaskID",
    "Name",
    "State",
    "UserName",
    "TimeUsed",
    "TimeLimit",
    "StartTime",
    "tres-alloc",
    "Partition",
    "NodeList",
    "STDOUT",
    "STDERR",
    "Reason",
    "WorkDir",
    "Command",
];
pub(super) const SEPARATOR: &str = "###turm###";

pub(crate) fn fetch_active(client: &SlurmClient, args: &[String]) -> Result<Vec<Job>, String> {
    let format = SQUEUE_FIELDS
        .map(|field| format!("{field}:0{SEPARATOR}"))
        .join(",");
    let output = client
        .output(Command::new(&client.squeue).args(args).args([
            "--array",
            "--noheader",
            &format!("--Format={format}"),
        ]))
        .map_err(|error| error.to_string())?;
    parser::parse_squeue_jobs(&output)
}

#[cfg(test)]
pub(super) fn parse_active_output(output: &str) -> Result<Vec<Job>, String> {
    parser::parse_squeue_jobs(output)
}

fn active_args(args: &[String]) -> Vec<String> {
    args.iter()
        .filter_map(|arg| {
            if arg == "--allusers" {
                None // squeue already includes other users.
            } else if let Some(nodes) = arg.strip_prefix("--node=") {
                Some(format!("--nodelist={nodes}"))
            } else if arg.starts_with("--state=") {
                None // State filters are partitioned between the two sources.
            } else {
                Some(arg.clone())
            }
        })
        .collect()
}

const TERMINAL_STATES: &str = "BOOT_FAIL,CANCELLED,COMPLETED,DEADLINE,FAILED,NODE_FAIL,OUT_OF_MEMORY,PREEMPTED,REVOKED,TIMEOUT";

pub(crate) struct JobQueries {
    pub(crate) active: Option<Vec<String>>,
    pub(crate) history: Option<Vec<String>>,
    pub(crate) history_user: Option<String>,
    pub(crate) retain_missing: bool,
}

impl JobQueries {
    pub(crate) fn new(args: &[String], user: Option<&str>) -> Self {
        let state_filter = args.iter().find_map(|arg| arg.strip_prefix("--state="));
        let mut active = active_args(args);
        if !args
            .iter()
            .any(|arg| arg == "--allusers" || arg.starts_with("--user="))
        {
            // sacct defaults to the current user; squeue defaults to everyone.
            active.push("--me".into());
        }
        let mut history = args
            .iter()
            .filter(|arg| *arg != "--allusers" && !arg.starts_with("--state="))
            .cloned()
            .collect::<Vec<_>>();
        let own_history = args.iter().any(|arg| arg == "--allusers");
        let history_user = own_history.then_some(user).flatten().map(str::to_string);
        if let Some(user) = &history_user {
            history.push(format!("--user={user}"));
        }
        let states = state_filter.unwrap_or("ALL").to_ascii_uppercase();
        let all = states == "ALL";
        let (terminal, live): (Vec<_>, Vec<_>) =
            states.split(',').partition(|state| is_historical(state));
        active.push(format!(
            "--states={}",
            if all { "all".into() } else { live.join(",") }
        ));
        history.push(format!(
            "--state={}",
            if all {
                TERMINAL_STATES.into()
            } else {
                terminal.join(",")
            }
        ));
        Self {
            active: (all || !live.is_empty()).then_some(active),
            history: ((all || !terminal.is_empty()) && (!own_history || user.is_some()))
                .then_some(history),
            history_user,
            // Disappearance under a mutable filter need not mean completion
            // (e.g. a renamed job no longer matches --name).
            retain_missing: all
                && (!own_history || user.is_some())
                && !args.iter().any(|arg| {
                    ["--name=", "--partition=", "--node="]
                        .iter()
                        .any(|prefix| arg.starts_with(prefix))
                }),
        }
    }
}
