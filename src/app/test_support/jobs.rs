use super::*;

pub fn parse_sacct_jobs(value: &str) -> Vec<Job> {
    crate::slurm::jobs::parse_sacct_jobs(value)
}

pub fn fetch_jobs_from(sacct: &Path, args: &[String]) -> Result<Vec<Job>, String> {
    crate::slurm::jobs::fetch_jobs_with(sacct, args)
}

pub fn filter_group_jobs(jobs: Vec<Job>, user: &str) -> Vec<Job> {
    crate::slurm::jobs::filter_group_jobs(jobs, user)
}
