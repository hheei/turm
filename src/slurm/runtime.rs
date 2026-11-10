use crossbeam::channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use super::{
    CommandFailure, JobSnapshot, JobSource, SlurmClient, SlurmCommand,
    jobs::{self, JobQueries},
    resources,
};
use crate::app::{AppMessage, PartitionResources};

pub(crate) enum SlurmEvent {
    Jobs(Result<JobSnapshot, String>),
    Resources(Result<Vec<PartitionResources>, String>),
    CommandFinished(Result<(), CommandFailure>),
}

pub(crate) struct SlurmHandle {
    commands: Sender<SlurmCommand>,
    // Dropping this disconnects every poller's stop receiver. No UI-side joins:
    // an in-flight external process may still be slow.
    _stop: Sender<()>,
}

impl SlurmHandle {
    pub(crate) fn start(
        app: Sender<AppMessage>,
        interval: Duration,
        history_interval: Duration,
        queries: JobQueries,
        client: SlurmClient,
    ) -> Self {
        let client = Arc::new(client);
        let (stop, stopped) = unbounded();
        for (source, args, cadence) in [
            (JobSource::Active, queries.active, interval),
            (JobSource::History, queries.history, history_interval),
        ] {
            let Some(args) = args else { continue };
            let client = Arc::clone(&client);
            poll(app.clone(), stopped.clone(), cadence, move || {
                let started_at = Instant::now();
                let jobs = match source {
                    JobSource::Active => jobs::fetch_active(&client, &args),
                    JobSource::History => jobs::fetch_history(&client, &args),
                };
                SlurmEvent::Jobs(jobs.map(|jobs| JobSnapshot {
                    source,
                    started_at,
                    jobs,
                }))
            });
        }
        let resources_client = Arc::clone(&client);
        poll(app.clone(), stopped.clone(), interval, move || {
            SlurmEvent::Resources(resources::fetch_resources(&resources_client))
        });
        let (commands, requests) = unbounded();
        thread::spawn(move || {
            while let Ok(request) = requests.recv() {
                if stopped
                    .try_recv()
                    .is_err_and(|error| error.is_disconnected())
                {
                    break;
                }
                let result = client.execute(request);
                if app
                    .send(AppMessage::Slurm(SlurmEvent::CommandFinished(result)))
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            commands,
            _stop: stop,
        }
    }

    pub(crate) fn submit(&self, command: SlurmCommand) -> Result<(), CommandFailure> {
        self.commands.send(command).map_err(|_| CommandFailure {
            command: "Slurm worker".into(),
            output: "Slurm worker has stopped".into(),
        })
    }

    /// A request sink for UI tests: exercise the actual submission boundary
    /// without launching processes or depending on worker scheduling.
    pub(crate) fn recording() -> (Self, Receiver<SlurmCommand>) {
        let (commands, requests) = unbounded();
        let (stop, _) = unbounded();
        (
            Self {
                commands,
                _stop: stop,
            },
            requests,
        )
    }
}

fn poll(
    app: Sender<AppMessage>,
    stopped: Receiver<()>,
    interval: Duration,
    mut fetch: impl FnMut() -> SlurmEvent + Send + 'static,
) {
    thread::spawn(move || {
        loop {
            if stopped
                .try_recv()
                .is_err_and(|error| error.is_disconnected())
            {
                break;
            }
            if app.send(AppMessage::Slurm(fetch())).is_err() {
                break;
            }
            match stopped.recv_timeout(interval.max(Duration::from_secs(1))) {
                Err(RecvTimeoutError::Timeout) => {}
                Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}
