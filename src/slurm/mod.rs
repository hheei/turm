//! Slurm I/O lives behind a client and a background runtime. The UI submits
//! owned requests and consumes completed snapshots; it never runs commands.
mod client;
mod commands;
pub(crate) mod jobs;
pub(crate) mod resources;
mod runtime;
mod snapshots;

pub(crate) use client::SlurmClient;
pub(crate) use commands::{CommandFailure, SlurmCommand};
pub(crate) use runtime::{SlurmEvent, SlurmHandle};
pub(crate) use snapshots::{JobSnapshot, JobSnapshots, JobSource};

#[cfg(test)]
mod tests;
