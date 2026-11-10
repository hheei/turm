use super::{
    jobs::{self, JobQueries},
    *,
};
use crate::test_support::test_job;
use std::time::{Duration, Instant};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn snapshot(source: JobSource, at: Instant, states: &[(usize, &str)]) -> JobSnapshot {
    JobSnapshot {
        source,
        started_at: at,
        jobs: states
            .iter()
            .map(|&(index, state)| {
                let mut job = test_job(index);
                job.state = state.into();
                job
            })
            .collect(),
    }
}

fn store() -> JobSnapshots {
    JobSnapshots::new(&JobQueries::new(&[], Some("chlo")))
}

#[test]
fn queries_split_sources_preserve_scope_and_limit_group_history_on_server() {
    let queries = JobQueries::new(
        &args(&[
            "--allusers",
            "--accounts=research",
            "--node=node01",
            "--clusters=alpha",
        ]),
        Some("chlo"),
    );
    let active = queries.active.unwrap();
    assert!(active.contains(&"--accounts=research".into()));
    assert!(active.contains(&"--clusters=alpha".into()));
    assert!(active.contains(&"--nodelist=node01".into()));
    assert!(!active.contains(&"--allusers".into()));
    assert!(!active.iter().any(|arg| arg.starts_with("--user=")));
    let history = queries.history.unwrap();
    assert!(history.contains(&"--user=chlo".into()));
    assert!(history.contains(&"--node=node01".into()));
    assert!(
        history
            .iter()
            .any(|arg| arg.starts_with("--state=BOOT_FAIL,"))
    );
    assert!(!history.contains(&"--allusers".into()));
}

#[test]
fn queries_partition_state_filters_instead_of_polling_irrelevant_sources() {
    let queries = JobQueries::new(&args(&["--state=R,FAILED,PD", "--user=alex"]), None);
    assert!(queries.active.unwrap().contains(&"--states=R,PD".into()));
    let history = queries.history.unwrap();
    assert!(history.contains(&"--state=FAILED".into()));
    assert!(history.contains(&"--user=alex".into()));
    assert!(
        JobQueries::new(&args(&["--state=CD"]), None)
            .active
            .is_none()
    );
    assert!(
        JobQueries::new(&[], Some("chlo"))
            .active
            .unwrap()
            .contains(&"--me".into())
    );
    let active_only = JobQueries::new(&args(&["--state=RUNNING"]), None);
    assert!(active_only.history.is_none());
    assert!(!active_only.retain_missing);
    assert!(!JobQueries::new(&args(&["--name=before-rename"]), None).retain_missing);
    // Never widen a group history query to everyone when identity is unavailable.
    assert!(
        JobQueries::new(&args(&["--allusers"]), None)
            .history
            .is_none()
    );
}

#[test]
fn special_exit_is_a_live_requeued_job_not_an_accounting_terminal_state() {
    for state in ["SPECIAL_EXIT", "SE"] {
        assert!(!jobs::is_historical(state));
        let queries = JobQueries::new(&args(&[&format!("--state={state}")]), None);
        assert!(
            queries
                .active
                .unwrap()
                .contains(&format!("--states={state}"))
        );
        assert!(queries.history.is_none());
    }
    let queries = JobQueries::new(&[], None);
    assert!(
        !queries
            .history
            .unwrap()
            .iter()
            .any(|arg| arg.contains("SPECIAL_EXIT"))
    );

    let mut snapshots = store();
    let jobs = snapshots.update(snapshot(
        JobSource::Active,
        Instant::now(),
        &[(0, "SPECIAL_EXIT")],
    ));
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].state, "SPECIAL_EXIT");
}

#[test]
fn accounting_handoff_preserves_missing_job_until_fresh_confirmation() {
    let mut store = store();
    let t = Instant::now();
    store.update(snapshot(JobSource::Active, t, &[(0, "RUNNING")]));
    store.update(snapshot(JobSource::History, t, &[(0, "COMPLETED")]));
    let jobs = store.update(snapshot(JobSource::Active, t + Duration::from_secs(1), &[]));
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].state, "RUNNING");
    let jobs = store.update(snapshot(
        JobSource::History,
        t + Duration::from_secs(2),
        &[(0, "COMPLETED")],
    ));
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].state, "COMPLETED");
}

#[test]
fn active_snapshot_wins_after_requeue_and_rejects_out_of_order_responses() {
    let mut store = store();
    let t = Instant::now();
    store.update(snapshot(JobSource::History, t, &[(0, "COMPLETED")]));
    store.update(snapshot(
        JobSource::Active,
        t + Duration::from_secs(2),
        &[(0, "PENDING")],
    ));
    let jobs = store.update(snapshot(
        JobSource::History,
        t + Duration::from_secs(3),
        &[(0, "COMPLETED")],
    ));
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].state, "PENDING");
    let jobs = store.update(snapshot(JobSource::Active, t, &[(0, "RUNNING")]));
    assert_eq!(jobs[0].state, "PENDING");
    store.update(snapshot(
        JobSource::History,
        t + Duration::from_secs(4),
        &[],
    ));
    let jobs = store.update(snapshot(JobSource::History, t, &[(1, "FAILED")]));
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].state, "PENDING");
}

#[test]
fn missing_group_jobs_and_state_filtered_jobs_do_not_linger() {
    let mut store = JobSnapshots::new(&JobQueries::new(
        &args(&["--allusers"]),
        Some("someone-else"),
    ));
    let t = Instant::now();
    store.update(snapshot(JobSource::Active, t, &[(0, "RUNNING")]));
    assert!(
        store
            .update(snapshot(JobSource::Active, t + Duration::from_secs(1), &[]))
            .is_empty()
    );
    let mut store = JobSnapshots::new(&JobQueries::new(&args(&["--state=PD"]), None));
    store.update(snapshot(JobSource::Active, t, &[(0, "PENDING")]));
    assert!(
        store
            .update(snapshot(JobSource::Active, t + Duration::from_secs(1), &[]))
            .is_empty()
    );
}

#[test]
fn completed_history_rolls_out_and_arrays_keep_distinct_identities() {
    let mut store = store();
    let t = Instant::now();
    let mut first = test_job(0);
    first.array_step = Some("1".into());
    first.display_id = "1000_1".into();
    let mut second = first.clone();
    second.array_step = Some("2".into());
    second.display_id = "1000_2".into();
    let jobs = store.update(JobSnapshot {
        source: JobSource::Active,
        started_at: t,
        jobs: vec![first, second],
    });
    assert_eq!(jobs.len(), 2);
    store.update(snapshot(JobSource::History, t, &[(1, "FAILED")]));
    let jobs = store.update(snapshot(
        JobSource::History,
        t + Duration::from_secs(1),
        &[],
    ));
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().all(|job| job.job_id == "1000"));
}

#[test]
fn squeue_fields_parse_paths_arrays_and_untruncated_command() {
    let separator = jobs::SEPARATOR;
    let fields = [
        "123",
        "100",
        "7",
        "array name",
        "RUNNING",
        "chlo",
        "12:37",
        "1:00:00",
        "2026-11-10T12:00:00",
        "cpu=4",
        "debug",
        "node01",
        "/work/out-%A_%a",
        "/work/err-%j",
        "None",
        "/work",
        "run --name a|b",
    ];
    let row = fields.join(separator) + separator + "\n";
    let jobs = jobs::parse_active_output(&row).unwrap();
    assert_eq!(jobs[0].id(), "100_7");
    assert_eq!(
        jobs[0].stdout.as_deref(),
        Some(std::path::Path::new("/work/out-100_7"))
    );
    assert_eq!(jobs[0].duration_seconds, Some(757));
    assert!(jobs[0].stderr.as_ref().unwrap().ends_with("err-123"));
    assert_eq!(jobs[0].command, "run --name a|b");
    assert!(jobs::parse_active_output("malformed response").is_err());
    assert!(jobs::parse_active_output("").unwrap().is_empty());
}

#[cfg(unix)]
mod processes {
    use super::*;
    use crate::AppMessage;
    use crossbeam::channel::{Receiver, unbounded};
    use std::{
        fs,
        io::{Read, Write},
        os::unix::fs::symlink,
        path::PathBuf,
        process::Command,
        sync::atomic::{AtomicUsize, Ordering},
        thread,
    };

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "turm-slurm-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            let fixture = Self(root);
            for program in ["sacct", "squeue", "sinfo", "scontrol", "scancel"] {
                fixture.script(program, "exit 0");
            }
            fixture
        }
        fn script(&self, name: &str, body: &str) {
            let path = self.0.join(name);
            fs::write(
                self.0.join(format!("{name}.body")),
                format!("#!/bin/sh\nset -eu\n{body}\n"),
            )
            .unwrap();
            if !path.exists() {
                symlink(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/fixtures/slurm-command.sh"),
                    path,
                )
                .unwrap();
            }
        }
        fn gate(&self, program: &str) -> Receiver<()> {
            let start = self.0.join(format!("{program}-started"));
            let release = self.0.join(format!("{program}-release"));
            assert!(
                Command::new("mkfifo")
                    .args([&start, &release])
                    .status()
                    .unwrap()
                    .success()
            );
            self.script(
                program,
                &format!(
                    "printf ready > '{}'\nread -r token < '{}'\nexit 0",
                    start.display(),
                    release.display()
                ),
            );
            let (tx, rx) = unbounded();
            thread::spawn(move || {
                let mut ready = String::new();
                fs::File::open(start)
                    .unwrap()
                    .read_to_string(&mut ready)
                    .unwrap();
                tx.send(()).unwrap();
            });
            rx
        }
        fn release(&self, program: &str) {
            fs::File::create(self.0.join(format!("{program}-release")))
                .unwrap()
                .write_all(b"continue\n")
                .unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn blocked_history_and_resources_do_not_block_active_queries_or_commands() {
        let fixture = Fixture::new();
        let history_started = fixture.gate("sacct");
        let resources_started = fixture.gate("sinfo");
        let (tx, rx) = unbounded();
        let handle = SlurmHandle::start(
            tx,
            Duration::from_secs(60),
            Duration::from_secs(60),
            JobQueries::new(&[], None),
            SlurmClient::in_directory(&fixture.0),
        );
        history_started
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        resources_started
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        handle
            .submit(SlurmCommand::Rename {
                id: "42".into(),
                value: "new name".into(),
            })
            .unwrap();
        let mut active = false;
        let mut command = false;
        while !active || !command {
            match rx.recv_timeout(Duration::from_secs(5)).unwrap() {
                AppMessage::Slurm(SlurmEvent::Jobs(Ok(snapshot))) => {
                    assert_eq!(snapshot.source, JobSource::Active);
                    active = true;
                }
                AppMessage::Slurm(SlurmEvent::CommandFinished(Ok(()))) => command = true,
                _ => panic!("unexpected response before gates released"),
            }
        }
        fixture.release("sacct");
        fixture.release("sinfo");
        drop(handle);
    }

    #[test]
    fn command_worker_preserves_order_and_reports_stderr_without_blocking_submit() {
        let fixture = Fixture::new();
        let started = fixture.gate("scancel");
        fixture.script("scontrol", "printf 'permission denied' >&2\nexit 1");
        let (tx, rx) = unbounded();
        let handle = SlurmHandle::start(
            tx,
            Duration::from_secs(60),
            Duration::from_secs(60),
            JobQueries::new(&args(&["--state=RUNNING"]), None),
            SlurmClient::in_directory(&fixture.0),
        );
        handle
            .submit(SlurmCommand::Cancel {
                id: "42".into(),
                signal: None,
            })
            .unwrap();
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        handle
            .submit(SlurmCommand::SetTimeLimit {
                id: "42".into(),
                value: "1:00:00".into(),
            })
            .unwrap();
        // The second request is accepted while the first process is gated.
        assert!(
            !rx.try_iter()
                .any(|event| matches!(event, AppMessage::Slurm(SlurmEvent::CommandFinished(_))))
        );
        fixture.release("scancel");
        let mut results = Vec::new();
        while results.len() < 2 {
            if let AppMessage::Slurm(SlurmEvent::CommandFinished(result)) =
                rx.recv_timeout(Duration::from_secs(5)).unwrap()
            {
                results.push(result);
            }
        }
        assert!(results.remove(0).is_ok());
        let error = results.remove(0).unwrap_err();
        assert!(error.command.contains("TimeLimit=1:00:00"));
        assert!(error.output.contains("permission denied"));
    }

    #[test]
    fn mock_slurm_live_query_uses_the_same_job_model_as_accounting() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/mock-slurm/bin");
        let client = SlurmClient::in_directory(root);
        let queries = JobQueries::new(&[], None);
        let live = jobs::fetch_active(&client, &queries.active.unwrap()).unwrap();
        assert_eq!(live.len(), 2);
        assert_eq!(live[0].id(), "1001");
        assert_eq!(live[0].state_compact, "R");
        assert_eq!(live[1].state_compact, "PD");
        assert!(live[0].command.contains("train.py"));
        assert!(live[0].stdout.as_ref().unwrap().ends_with("job-1001.out"));
        let mut store = store();
        let t = Instant::now();
        store.update(JobSnapshot {
            source: JobSource::Active,
            started_at: t,
            jobs: live,
        });
        let history = jobs::fetch_history(&client, &queries.history.unwrap()).unwrap();
        let merged = store.update(JobSnapshot {
            source: JobSource::History,
            started_at: t,
            jobs: history,
        });
        assert_eq!(merged.len(), 3);
        assert_eq!(merged.iter().filter(|job| job.id() == "1001").count(), 1);
    }

    #[test]
    fn user_text_is_passed_as_one_literal_argument() {
        let fixture = Fixture::new();
        let arguments = fixture.0.join("arguments");
        fixture.script(
            "scontrol",
            &format!("printf '%s\\n' \"$@\" > '{}'", arguments.display()),
        );
        let value = "name with spaces; $(touch should-never-run)";
        SlurmClient::in_directory(&fixture.0)
            .execute(SlurmCommand::Rename {
                id: "42".into(),
                value: value.into(),
            })
            .unwrap();
        assert_eq!(
            fs::read_to_string(arguments).unwrap(),
            format!("update\nJobId=42\nName={value}\n")
        );
    }
}
