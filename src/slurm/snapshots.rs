use std::{collections::BTreeMap, time::Instant};

use super::jobs::is_historical;
use crate::app::Job;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JobSource {
    Active,
    History,
}

pub(crate) struct JobSnapshot {
    pub(crate) source: JobSource,
    /// Query start, rather than arrival time: a slow accounting response must
    /// not overwrite a newer controller observation after requeue.
    pub(crate) started_at: Instant,
    pub(crate) jobs: Vec<Job>,
}

struct ActiveJob {
    job: Job,
    missing_since: Option<Instant>,
}

#[derive(Default)]
pub(crate) struct JobSnapshots {
    active: BTreeMap<String, ActiveJob>,
    history: BTreeMap<String, Job>,
    active_at: Option<Instant>,
    history_at: Option<Instant>,
    history_user: Option<String>,
    retain_missing: bool,
}

impl JobSnapshots {
    pub(crate) fn new(queries: &super::jobs::JobQueries) -> Self {
        Self {
            history_user: queries.history_user.clone(),
            retain_missing: queries.retain_missing,
            ..Self::default()
        }
    }

    pub(crate) fn update(&mut self, snapshot: JobSnapshot) -> Vec<Job> {
        let JobSnapshot {
            source,
            started_at,
            jobs,
        } = snapshot;
        match source {
            JobSource::Active if self.active_at.is_none_or(|last| started_at >= last) => {
                self.active_at = Some(started_at);
                for entry in self.active.values_mut() {
                    entry.missing_since.get_or_insert(started_at);
                }
                for job in jobs.into_iter().filter(|job| !is_historical(&job.state)) {
                    self.active.insert(
                        job.id().to_string(),
                        ActiveJob {
                            job,
                            missing_since: None,
                        },
                    );
                }
            }
            JobSource::History if self.history_at.is_none_or(|last| started_at >= last) => {
                self.history_at = Some(started_at);
                self.history = jobs
                    .into_iter()
                    .filter(|job| is_historical(&job.state))
                    .map(|job| (job.id().to_string(), job))
                    .collect();
            }
            _ => {}
        }
        // Keep missing allocations until a subsequent accounting query confirms
        // their terminal state. A failed query never clears an existing snapshot.
        self.active.retain(|id, entry| {
            if entry.missing_since.is_some()
                && (!self.retain_missing
                    || self
                        .history_user
                        .as_ref()
                        .is_some_and(|user| *user != entry.job.user))
            {
                return false;
            }
            !entry.missing_since.is_some_and(|missing| {
                self.history_at.is_some_and(|history| history >= missing)
                    && self.history.contains_key(id)
            })
        });
        self.history
            .iter()
            .filter(|(id, _)| !self.active.contains_key(*id))
            .map(|(_, job)| job.clone())
            .chain(self.active.values().map(|entry| entry.job.clone()))
            .collect()
    }
}
