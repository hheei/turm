use super::*;

pub fn parse_sacct_jobs(value: &str) -> Vec<Job> {
    crate::job_watcher::parse_sacct_jobs(value)
}

pub fn fetch_jobs_from(sacct: &Path, args: &[String]) -> Result<Vec<Job>, String> {
    crate::job_watcher::fetch_jobs_with(sacct, args)
}

pub fn filter_group_jobs(jobs: Vec<Job>, user: &str) -> Vec<Job> {
    crate::job_watcher::filter_group_jobs(jobs, user)
}
