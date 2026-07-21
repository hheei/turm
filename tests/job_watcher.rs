use std::path::{Path, PathBuf};

use turm::test_support::{fetch_jobs_from, filter_group_jobs, parse_sacct_jobs};

const SAMPLE: &str = concat!(
    "42|42|running-job|RUNNING|chlo|00:12:37|01:00:00|2026-07-20T12:00:00|cpu=4|debug|node-01|/tmp/job-42.out|/tmp/job-42.err|None|/work/chlo|run.sh\n",
    "43|43|finished-job|COMPLETED|chlo|00:03:21|00:10:00|2026-07-19T09:00:00|cpu=1|debug|node-02|||None|/work/chlo|\n",
    "44|44|cancelled-job|CANCELLED by 2028|chlo|00:00:03|00:10:00|Unknown|cpu=1|debug||||JobHeldUser|/work/chlo|\n",
);

#[test]
fn parses_active_and_historical_sacct_allocations() {
    let jobs = parse_sacct_jobs(SAMPLE);

    assert_eq!(jobs.len(), 3);
    assert_eq!(jobs[0].state_compact, "R");
    assert_eq!(jobs[1].state, "COMPLETED");
    assert_eq!(jobs[1].state_compact, "CD");
    assert_eq!(jobs[2].state_compact, "CA");
    assert_eq!(jobs[2].reason.as_deref(), Some("JobHeldUser"));
}

#[test]
fn parses_array_ids_and_resolves_default_output_path() {
    let jobs = parse_sacct_jobs(
        "50_7|50_7|array-job|PENDING|chlo|00:00:00|01:00:00|Unknown|cpu=1|debug||||Resources|/work/chlo|\n",
    );

    assert_eq!(jobs[0].job_id, "50");
    assert_eq!(jobs[0].array_id, "50");
    assert_eq!(jobs[0].array_step.as_deref(), Some("7"));
    assert_eq!(
        jobs[0].stdout,
        Some(PathBuf::from("/work/chlo/slurm-50_7.out"))
    );
    assert_eq!(
        jobs[0].stderr,
        Some(PathBuf::from("/work/chlo/slurm-50_7.out"))
    );
}

#[test]
fn ignores_malformed_sacct_rows() {
    assert!(parse_sacct_jobs("not|enough|fields\n").is_empty());
}

#[test]
fn fetches_jobs_from_mock_sacct_without_global_path_changes() {
    let sacct = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/mock-slurm/bin/sacct");
    let jobs = fetch_jobs_from(&sacct, &[]).expect("mock sacct should succeed");

    assert_eq!(jobs.len(), 3);
    assert!(jobs.iter().any(|job| job.state == "PENDING"));
    assert!(jobs.iter().any(|job| job.state == "RUNNING"));
    assert!(jobs.iter().any(|job| job.state == "COMPLETED"));
}

#[test]
fn group_scope_keeps_all_active_jobs_but_only_own_history() {
    let jobs = parse_sacct_jobs(concat!(
        "42|42|mine-running|RUNNING|chlo|00:01:00|01:00:00|2026-07-20T12:00:00|cpu=1|debug||||None|/work/chlo|\n",
        "43|43|group-pending|PENDING|alex|00:00:00|01:00:00|Unknown|cpu=1|debug||||None|/work/alex|\n",
        "44|44|mine-finished|COMPLETED|chlo|00:01:00|01:00:00|2026-07-20T11:00:00|cpu=1|debug||||None|/work/chlo|\n",
        "45|45|group-finished|FAILED|alex|00:01:00|01:00:00|2026-07-20T11:00:00|cpu=1|debug||||None|/work/alex|\n",
    ));

    let jobs = filter_group_jobs(jobs, "chlo");
    assert_eq!(
        jobs.iter()
            .map(|job| job.job_id.as_str())
            .collect::<Vec<_>>(),
        ["42", "43", "44"]
    );
}
