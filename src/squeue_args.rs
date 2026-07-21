use clap::Args;
use std::process::Command;

#[derive(Args, Debug)]
pub struct SqueueArgs {
    /// Comma-separated list of accounts to view.
    #[arg(short = 'A', long)]
    account: Option<String>,

    /// Report federated information if a member of one.
    #[arg(long)]
    federation: bool,

    /// Comma-separated list of job IDs to view.
    #[arg(short, long, value_name = "JOBID")]
    job: Option<String>,

    /// Report jobs only from the local cluster.
    #[arg(long)]
    local: bool,

    /// Cluster to query.
    #[arg(short = 'M', long)]
    clusters: Option<String>,

    /// Show only your own jobs.
    #[arg(long)]
    me: bool,

    /// Comma-separated list of job names to view.
    #[arg(short = 'n', long)]
    name: Option<String>,

    /// Keep resource units in their original form.
    #[arg(long)]
    noconvert: bool,

    /// Comma-separated list of partitions to view.
    #[arg(short, long)]
    partition: Option<String>,

    /// Comma-separated list of job states; ALL leaves states unfiltered.
    #[arg(short = 't', long)]
    states: Option<String>,

    /// Comma-separated list of users to view.
    #[arg(short = 'u', long)]
    user: Option<String>,

    /// Comma-separated list of nodes to view.
    #[arg(short = 'w', long, value_name = "NODES")]
    nodelist: Option<String>,
}

impl SqueueArgs {
    pub fn to_vec(&self) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(account) = &self.account {
            args.push(format!("--accounts={account}"));
        } else if !self.me
            && self.user.is_none()
            && let Some(account) = primary_group()
        {
            args.push(format!("--accounts={account}"));
        }
        if self.federation {
            args.push("--federation".to_string());
        }
        if let Some(job) = &self.job {
            args.push(format!("--jobs={job}"));
        }
        if self.local {
            args.push("--local".to_string());
        }
        if let Some(clusters) = &self.clusters {
            args.push(format!("--clusters={clusters}"));
        }
        if !self.me && self.user.is_none() {
            args.push("--allusers".to_string());
        }
        if let Some(name) = &self.name {
            args.push(format!("--name={name}"));
        }
        if self.noconvert {
            args.push("--noconvert".to_string());
        }
        if let Some(partition) = &self.partition {
            args.push(format!("--partition={partition}"));
        }
        if let Some(states) = &self.states
            && !states.eq_ignore_ascii_case("all")
        {
            args.push(format!("--state={states}"));
        }
        if let Some(user) = &self.user {
            args.push(format!("--user={user}"));
        }
        if let Some(nodelist) = &self.nodelist {
            args.push(format!("--node={nodelist}"));
        }
        args
    }
}

fn primary_group() -> Option<String> {
    let output = Command::new("id").arg("-gn").output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|group| !group.is_empty())
}
