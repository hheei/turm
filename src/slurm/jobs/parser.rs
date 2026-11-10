use std::path::PathBuf;

use regex::Regex;

use crate::app::Job;

const FIELD_COUNT: usize = 16;
const SLURM_NO_ARRAY_VALUE: &str = "4294967294";

pub(super) fn parse_sacct_jobs(output: &str) -> Vec<Job> {
    parse_sacct_jobs_for_user(output, None)
}

pub(super) fn parse_sacct_jobs_for_user(output: &str, user: Option<&str>) -> Vec<Job> {
    output
        .lines()
        .filter_map(|line| parse_sacct_job(line, user))
        .collect()
}

fn parse_sacct_job(line: &str, filter_user: Option<&str>) -> Option<Job> {
    let parts = line.trim().splitn(FIELD_COUNT, '|').collect::<Vec<_>>();
    parse_job_fields(&parts, filter_user)
}

pub(super) fn parse_squeue_jobs(output: &str) -> Result<Vec<Job>, String> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parts = line.trim().split(super::SEPARATOR).collect::<Vec<_>>();
            if parts.len() != FIELD_COUNT + 2 || !parts.last().unwrap().is_empty() {
                return Err("invalid squeue row: unexpected field count".to_string());
            }
            let id = if normalize_optional(parts[2]).is_some() && parts[2] != SLURM_NO_ARRAY_VALUE {
                format!("{}_{}", parts[1].trim(), parts[2].trim())
            } else {
                parts[0].trim().to_string()
            };
            let mut fields = vec![id.as_str(), parts[0]];
            fields.extend_from_slice(&parts[3..parts.len() - 1]);
            parse_job_fields(&fields, None)
                .ok_or_else(|| "invalid squeue row: missing job identity".to_string())
        })
        .collect()
}

fn parse_job_fields(parts: &[&str], filter_user: Option<&str>) -> Option<Job> {
    if parts.len() != FIELD_COUNT {
        return None;
    }

    let id = nonempty(parts[0]).or_else(|| nonempty(parts[1]))?;
    let raw_id = nonempty(parts[1]).unwrap_or(id);
    let (job_id, array_id, array_step) = parse_job_id(id);
    let display_id = match array_step.as_ref() {
        Some(array_step) => format!("{array_id}_{array_step}"),
        None => job_id.clone(),
    };
    let duration_seconds = parse_slurm_duration(parts[5].trim());
    let state = parts[3].trim();
    let user = parts[4].trim();
    let name = parts[2].trim();
    if filter_user.is_some_and(|filter_user| is_historical(state) && user != filter_user) {
        return None;
    }
    let nodelist = normalize_optional(parts[10]).unwrap_or_default();
    let working_dir = normalize_optional(parts[14]).unwrap_or_default();
    let stdout = normalize_optional(parts[11]).unwrap_or_default();
    let stderr = normalize_optional(parts[12]).unwrap_or_default();
    let array_task_id = array_step.as_deref().unwrap_or("N/A");
    let stdout = resolve_path(
        stdout,
        &array_id,
        array_task_id,
        raw_id,
        nodelist,
        user,
        name,
        working_dir,
    );
    let stderr = resolve_path(
        stderr,
        &array_id,
        array_task_id,
        raw_id,
        nodelist,
        user,
        name,
        working_dir,
    );

    Some(Job {
        job_id,
        array_id: array_id.clone(),
        array_step,
        display_id,
        duration_seconds,
        name: name.to_string(),
        state: state.to_string(),
        state_compact: compact_state(state),
        reason: normalize_optional(parts[13]).map(str::to_string),
        user: user.to_string(),
        time: parts[5].trim().to_string(),
        time_limit: parts[6].trim().to_string(),
        start_time: normalize_optional(parts[7]).unwrap_or("N/A").to_string(),
        tres: parts[8].trim().to_string(),
        partition: parts[9].trim().to_string(),
        nodelist: nodelist.to_string(),
        stdout,
        stderr,
        workdir: nonempty(working_dir).map(PathBuf::from),
        command: parts[15].trim().to_string(),
    })
}

fn parse_job_id(id: &str) -> (String, String, Option<String>) {
    if let Some((array_id, array_step)) = id.split_once('_')
        && !array_id.is_empty()
        && !array_step.is_empty()
    {
        return (
            array_id.to_string(),
            array_id.to_string(),
            Some(array_step.to_string()),
        );
    }

    (id.to_string(), id.to_string(), None)
}

fn compact_state(state: &str) -> String {
    match state.split_whitespace().next().unwrap_or_default() {
        "BOOT_FAIL" => "BF",
        "CANCELLED" => "CA",
        "COMPLETED" => "CD",
        "COMPLETING" => "CG",
        "CONFIGURING" => "CF",
        "DEADLINE" => "DL",
        "FAILED" => "F",
        "NODE_FAIL" => "NF",
        "OUT_OF_MEMORY" => "OOM",
        "PENDING" => "PD",
        "PREEMPTED" => "PR",
        "REQUEUED" => "RQ",
        "RESIZING" => "RS",
        "REVOKED" => "RV",
        "RUNNING" => "R",
        "SPECIAL_EXIT" => "SE",
        "STOPPED" => "ST",
        "SUSPENDED" => "S",
        "TIMEOUT" => "TO",
        value => value,
    }
    .to_string()
}

fn normalize_optional(value: &str) -> Option<&str> {
    match value.trim() {
        "" | "None" | "Unknown" | "(null)" | "N/A" => None,
        value => Some(value),
    }
}

fn nonempty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}
fn parse_slurm_duration(value: &str) -> Option<u64> {
    let (days, rest) = match value.split_once('-') {
        Some((days, rest)) => (days.parse::<u64>().ok()?, rest),
        None => (0, value),
    };
    let mut parts = rest.split(':');
    let first = parts.next()?;
    let second = parts.next()?;
    let third = parts.next();
    if parts.next().is_some() {
        return None;
    }
    let (hours, minutes, seconds) = match third {
        Some(seconds) => (
            first.parse::<u64>().ok()?,
            second.parse::<u64>().ok()?,
            seconds.parse::<u64>().ok()?,
        ),
        None => (0, first.parse::<u64>().ok()?, second.parse::<u64>().ok()?),
    };
    Some(days * 86_400 + hours * 3_600 + minutes * 60 + seconds)
}

#[allow(clippy::too_many_arguments)]
fn resolve_path(
    path: &str,
    array_master: &str,
    array_id: &str,
    id: &str,
    host: &str,
    user: &str,
    name: &str,
    working_dir: &str,
) -> Option<PathBuf> {
    lazy_static::lazy_static! {
        static ref RE: Regex = Regex::new(r"%(%|A|a|J|j|N|n|s|t|u|x)").unwrap();
    }

    let array_id = if array_id == "N/A" {
        SLURM_NO_ARRAY_VALUE
    } else {
        array_id
    };
    let mut path = if path.is_empty() {
        if array_id == SLURM_NO_ARRAY_VALUE {
            "slurm-%J.out".to_string()
        } else {
            "slurm-%A_%a.out".to_string()
        }
    } else {
        path.to_string()
    };
    if !path.contains('%') {
        return Some(PathBuf::from(working_dir).join(path));
    }

    for capture in RE
        .captures_iter(&path.clone())
        .collect::<Vec<_>>()
        .iter()
        .rev()
    {
        let matched = capture.get(0).unwrap();
        let replacement = match matched.as_str() {
            "%%" => "%",
            "%A" => array_master,
            "%a" => array_id,
            "%J" | "%j" => id,
            "%N" => host.split(',').next().unwrap_or(host),
            "%n" | "%t" => "0",
            "%s" => "batch",
            "%u" => user,
            "%x" => name,
            _ => unreachable!(),
        };
        path.replace_range(matched.range(), replacement);
    }

    Some(PathBuf::from(working_dir).join(path))
}
use super::is_historical;
