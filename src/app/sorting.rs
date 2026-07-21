use super::*;

impl SortDirection {
    fn toggled(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }

    fn indicator(self) -> &'static str {
        match self {
            Self::Asc => "▲",
            Self::Desc => "▼",
        }
    }
}

impl App {
    pub(super) fn sort_indicator(&self, field: JobSortField) -> &'static str {
        if self.job_sort_field == field {
            self.job_sort_direction.indicator()
        } else {
            ""
        }
    }

    pub(super) fn update_job_sort(&mut self, field: JobSortField) {
        let selected_id = self.selected_job_id();
        let fallback_index = self.job_list_state.selected();

        if self.job_sort_field == field {
            self.job_sort_direction = self.job_sort_direction.toggled();
        } else {
            self.job_sort_field = field;
            self.job_sort_direction = SortDirection::Desc;
        }

        self.sort_jobs();
        self.restore_selection_by_job_id(selected_id, fallback_index);
    }

    pub(super) fn sort_jobs(&mut self) {
        let sort_field = self.job_sort_field;
        let sort_direction = self.job_sort_direction;

        self.jobs.sort_by(|left, right| {
            let completed_order = (left.state_compact.eq_ignore_ascii_case("CD"))
                .cmp(&right.state_compact.eq_ignore_ascii_case("CD"));
            let ordering = compare_jobs_by_field(left, right, sort_field);
            completed_order.then_with(|| match sort_direction {
                SortDirection::Asc => ordering,
                SortDirection::Desc => ordering.reverse(),
            })
        });
    }

    pub(super) fn restore_selection_by_job_id(
        &mut self,
        selected_id: Option<String>,
        fallback_index: Option<usize>,
    ) {
        let visible_job_indices = self.visible_job_indices();
        if visible_job_indices.is_empty() {
            self.job_list_state.select(None);
            return;
        }

        if let Some(selected_id) = selected_id {
            if let Some(index) = visible_job_indices
                .iter()
                .position(|&job_index| self.jobs[job_index].id() == selected_id)
            {
                self.job_list_state.select(Some(index));
                return;
            }
        }

        if let Some(index) = fallback_index {
            self.job_list_state
                .select(Some(index.min(visible_job_indices.len() - 1)));
        } else {
            self.job_list_state.select_first();
        }
    }
}

fn compare_jobs_by_field(left: &Job, right: &Job, field: JobSortField) -> Ordering {
    match field {
        JobSortField::State => cmp_ignore_ascii_case(&left.state_compact, &right.state_compact),
        JobSortField::Partition => cmp_ignore_ascii_case(&left.partition, &right.partition),
        JobSortField::Id => compare_job_identity(left, right),
        JobSortField::Name => cmp_ignore_ascii_case(&left.name, &right.name),
        JobSortField::User => cmp_ignore_ascii_case(&left.user, &right.user),
        JobSortField::Time => compare_job_time(left, right),
    }
    .then_with(|| compare_job_identity(left, right))
}

fn compare_job_identity(left: &Job, right: &Job) -> Ordering {
    compare_numeric_string(&left.job_id, &right.job_id)
        .then_with(|| {
            compare_optional_numeric_string(left.array_step.as_deref(), right.array_step.as_deref())
        })
        .then_with(|| cmp_ignore_ascii_case(&left.id(), &right.id()))
}

fn compare_job_time(left: &Job, right: &Job) -> Ordering {
    compare_optional_u64(
        parse_slurm_duration(&left.time),
        parse_slurm_duration(&right.time),
    )
    .then_with(|| cmp_ignore_ascii_case(&left.time, &right.time))
}

fn compare_numeric_string(left: &str, right: &str) -> Ordering {
    match (left.parse::<u64>(), right.parse::<u64>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => cmp_ignore_ascii_case(left, right),
    }
}

fn compare_optional_numeric_string(left: Option<&str>, right: Option<&str>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => compare_numeric_string(left, right),
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_u64(left: Option<u64>, right: Option<u64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn cmp_ignore_ascii_case(left: &str, right: &str) -> Ordering {
    left.bytes()
        .map(|byte| byte.to_ascii_lowercase())
        .cmp(right.bytes().map(|byte| byte.to_ascii_lowercase()))
}

fn parse_slurm_duration(value: &str) -> Option<u64> {
    let (days, rest) = match value.split_once('-') {
        Some((days, rest)) => (days.parse::<u64>().ok()?, rest),
        None => (0, value),
    };

    let mut parts = rest.split(':');
    let first = parts.next()?;
    let second = parts.next()?;
    let third = parts.next();
    if parts.next().is_some() {
        return None;
    }

    let (hours, minutes, seconds) = match third {
        Some(seconds) => (
            first.parse::<u64>().ok()?,
            second.parse::<u64>().ok()?,
            seconds.parse::<u64>().ok()?,
        ),
        None => (0, first.parse::<u64>().ok()?, second.parse::<u64>().ok()?),
    };

    Some(days * 86_400 + hours * 3_600 + minutes * 60 + seconds)
}
