impl JobFilterField {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "job" => Some(Self::Job),
            "id" => Some(Self::Id),
            "name" => Some(Self::Name),
            "user" => Some(Self::User),
            "partition" | "part" => Some(Self::Partition),
            "state" | "st" => Some(Self::State),
            "time" => Some(Self::Time),
            _ => None,
        }
    }
}

impl JobFilter {
    fn parse(query: &str) -> Self {
        let query = query.trim();
        if query.is_empty() {
            return Self::None;
        }

        if let Some((field, value)) = query.split_once(':') {
            if let Some(field) = JobFilterField::parse(field) {
                return Self::Field(field, value.trim().to_lowercase());
            }
        }

        Self::FreeText(query.to_lowercase())
    }

    fn matches(&self, job: &Job) -> bool {
        match self {
            Self::None => true,
            Self::FreeText(query) => {
                contains_case_insensitive(&job.state, query)
                    || contains_case_insensitive(&job.state_compact, query)
                    || contains_case_insensitive(&job.partition, query)
                    || contains_case_insensitive(&job.id(), query)
                    || contains_case_insensitive(&job.name, query)
                    || contains_case_insensitive(&job.user, query)
                    || contains_case_insensitive(&job.time, query)
            }
            Self::Field(field, query) => match field {
                JobFilterField::Job => {
                    contains_case_insensitive(&job.id(), query)
                        || contains_case_insensitive(&job.name, query)
                }
                JobFilterField::Id => contains_case_insensitive(&job.id(), query),
                JobFilterField::Name => contains_case_insensitive(&job.name, query),
                JobFilterField::User => contains_case_insensitive(&job.user, query),
                JobFilterField::Partition => contains_case_insensitive(&job.partition, query),
                JobFilterField::State => {
                    contains_case_insensitive(&job.state, query)
                        || contains_case_insensitive(&job.state_compact, query)
                }
                JobFilterField::Time => contains_case_insensitive(&job.time, query),
            },
        }
    }
}

fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(needle)
}

impl App {
    pub(in crate::app) fn visible_job_indices(&self) -> Vec<usize> {
        let filter = JobFilter::parse(&self.active_filter);
        self.jobs
            .iter()
            .enumerate()
            .filter_map(|(index, job)| filter.matches(job).then_some(index))
            .collect()
    }

    pub(in crate::app) fn apply_job_filter(&mut self, filter: &str) {
        let selected_id = self.selected_job_id();
        let fallback_index = self.job_list_state.selected();

        self.active_filter = filter.trim().to_string();
        self.restore_selection_by_job_id(selected_id, fallback_index);
    }
}
use super::*;
