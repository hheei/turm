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
                    || contains_case_insensitive(job.id(), query)
                    || contains_case_insensitive(&job.name, query)
                    || contains_case_insensitive(&job.user, query)
                    || contains_case_insensitive(&job.time, query)
            }
            Self::Field(field, query) => match field {
                JobFilterField::Job => {
                    contains_case_insensitive(job.id(), query)
                        || contains_case_insensitive(&job.name, query)
                }
                JobFilterField::Id => contains_case_insensitive(job.id(), query),
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
    if haystack.is_ascii() && needle.is_ascii() {
        return haystack.as_bytes().windows(needle.len()).any(|window| {
            window
                .iter()
                .zip(needle.bytes())
                .all(|(left, right)| left.to_ascii_lowercase() == right.to_ascii_lowercase())
        });
    }
    haystack.to_lowercase().contains(needle)
}

impl App {
    pub(in crate::app) fn visible_job_indices(&self) -> std::cell::Ref<'_, Vec<usize>> {
        let cache_is_current =
            self.visible_job_cache
                .borrow()
                .as_ref()
                .is_some_and(|(filter, generation, _)| {
                    filter == &self.active_filter && *generation == self.jobs_generation
                });
        if !cache_is_current {
            let filter = JobFilter::parse(&self.active_filter);
            let indices = self
                .jobs
                .iter()
                .enumerate()
                .filter_map(|(index, job)| filter.matches(job).then_some(index))
                .collect();
            *self.visible_job_cache.borrow_mut() =
                Some((self.active_filter.clone(), self.jobs_generation, indices));
        }
        std::cell::Ref::map(self.visible_job_cache.borrow(), |cache| {
            &cache.as_ref().unwrap().2
        })
    }

    pub(in crate::app) fn invalidate_visible_job_cache(&mut self) {
        self.jobs_generation = self.jobs_generation.wrapping_add(1);
        *self.visible_job_cache.get_mut() = None;
    }

    pub(in crate::app) fn apply_job_filter(&mut self, filter: &str) {
        let selected_id = self.selected_job_id();
        let fallback_index = self.job_list_state.selected();
        self.active_filter = filter.trim().to_string();
        self.invalidate_visible_job_cache();
        self.restore_selection_by_job_id(selected_id, fallback_index);
    }
}

use super::*;
