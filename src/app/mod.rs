use crossbeam::{
    channel::{Receiver, TryRecvError, unbounded},
    select,
};
use itertools::Either;
use std::{
    cmp::{Ordering, min},
    iter::once,
    path::PathBuf,
    process::Command,
    time::Duration,
};

use crate::file_watcher::{FileWatcherError, FileWatcherHandle};
use crate::job_watcher::JobWatcherHandle;

use crossterm::event::{Event, KeyCode, KeyEvent, MouseButton, MouseEventKind};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState, Wrap,
    },
};
use std::io;
use tui_input::{Input, backend::crossterm::EventHandler};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Jobs,
    Details,
    Log,
}

pub enum Dialog {
    ConfirmCancelJob {
        id: String,
        name: String,
        details: Vec<String>,
        signal: Option<String>,
    },
    EditTimeLimit {
        id: String,
        input: Input,
    },
    FilterJobs {
        input: Input,
    },
    CopyJobOutputDirectory {
        dir_url: String,
        dir_name: String,
    },
    CommandError {
        command: String,
        output: String,
    },
}

struct CommandFailure {
    command: String,
    output: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAnchor {
    Top,
    Bottom,
}

#[derive(Clone, Copy, Default)]
pub enum OutputFileView {
    #[default]
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobSortField {
    State,
    Partition,
    Id,
    Name,
    User,
    Time,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Copy)]
enum JobFilterField {
    Job,
    Id,
    Name,
    User,
    Partition,
    State,
    Time,
}

enum JobFilter {
    None,
    FreeText(String),
    Field(JobFilterField, String),
}

pub struct App {
    focus: Focus,
    dialog: Option<Dialog>,
    jobs: Vec<Job>,
    active_filter: String,
    job_list_state: TableState,
    job_sort_field: JobSortField,
    job_sort_direction: SortDirection,
    job_output: Result<String, FileWatcherError>,
    job_output_anchor: ScrollAnchor,
    job_output_offset: u16,
    job_output_wrap: bool,
    _job_watcher: JobWatcherHandle,
    job_output_watcher: FileWatcherHandle,
    // sender: Sender<AppMessage>,
    receiver: Receiver<AppMessage>,
    input_receiver: Receiver<std::io::Result<Event>>,
    output_file_view: OutputFileView,
    job_list_height: u16,
    job_list_area: Rect,
    job_details_area: Rect,
    job_output_area: Rect,
    pending_input_event: Option<Event>,
    pending_clipboard_copy: Option<String>,
}

pub struct Job {
    pub job_id: String,
    pub array_id: String,
    pub array_step: Option<String>,
    pub name: String,
    pub state: String,
    pub state_compact: String,
    pub reason: Option<String>,
    pub user: String,
    pub time: String,
    pub time_limit: String,
    pub start_time: String,
    pub tres: String,
    pub partition: String,
    pub nodelist: String,
    pub stdout: Option<PathBuf>,
    pub stderr: Option<PathBuf>,
    pub command: String,
}

impl Job {
    fn id(&self) -> String {
        match self.array_step.as_ref() {
            Some(array_step) => format!("{}_{}", self.array_id, array_step),
            None => self.job_id.clone(),
        }
    }
}

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

pub enum AppMessage {
    Jobs(Vec<Job>),
    JobOutput(Result<String, FileWatcherError>),
    Key(KeyEvent),
    MouseClick {
        column: u16,
        row: u16,
    },
    MouseWheel {
        target: MouseScrollTarget,
        direction: MouseWheelDirection,
        amount: u16,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MouseWheelDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MouseScrollTarget {
    Jobs,
    Output,
}

const DIALOG_WIDTH: u16 = 80;

impl App {
    pub fn new(
        input_receiver: Receiver<std::io::Result<Event>>,
        slurm_refresh_rate: u64,
        file_refresh_rate: u64,
        squeue_args: Vec<String>,
    ) -> App {
        let (sender, receiver) = unbounded();
        Self {
            focus: Focus::Jobs,
            dialog: None,
            jobs: Vec::new(),
            active_filter: String::new(),
            _job_watcher: JobWatcherHandle::new(
                sender.clone(),
                Duration::from_secs(slurm_refresh_rate),
                squeue_args,
            ),
            job_list_state: TableState::new(),
            job_sort_field: JobSortField::Time,
            job_sort_direction: SortDirection::Asc,
            job_output: Ok("".to_string()),
            job_output_anchor: ScrollAnchor::Bottom,
            job_output_offset: 0,
            job_output_wrap: false,
            job_output_watcher: FileWatcherHandle::new(
                sender.clone(),
                Duration::from_secs(file_refresh_rate),
            ),
            // sender,
            // sender,
            receiver,
            input_receiver,
            output_file_view: OutputFileView::default(),
            job_list_height: 0,
            job_list_area: Rect::default(),
            job_details_area: Rect::default(),
            job_output_area: Rect::default(),
            pending_input_event: None,
            pending_clipboard_copy: None,
        }
    }
}

impl App {
    pub(super) fn visible_job_indices(&self) -> Vec<usize> {
        let filter = JobFilter::parse(&self.active_filter);
        self.jobs
            .iter()
            .enumerate()
            .filter_map(|(index, job)| filter.matches(job).then_some(index))
            .collect()
    }

    pub(super) fn apply_job_filter(&mut self, filter: &str) {
        let selected_id = self.selected_job_id();
        let fallback_index = self.job_list_state.selected();

        self.active_filter = filter.trim().to_string();
        self.restore_selection_by_job_id(selected_id, fallback_index);
    }
}

mod commands;
mod events;
mod render;
mod sorting;
#[cfg(test)]
mod tests;
