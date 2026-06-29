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

pub enum Focus {
    Jobs,
}

pub enum Dialog {
    ConfirmCancelJob(String),
    SelectCancelSignal { id: String, selected_signal: usize },
    EditTimeLimit { id: String, input: Input },
    CommandError { command: String, output: String },
}

struct CommandFailure {
    command: String,
    output: String,
}

#[derive(Clone, Copy)]
pub enum ScrollAnchor {
    Top,
    Bottom,
}

#[derive(Default)]
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

pub struct App {
    focus: Focus,
    dialog: Option<Dialog>,
    jobs: Vec<Job>,
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
    job_output_area: Rect,
    pending_input_event: Option<Event>,
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

pub enum AppMessage {
    Jobs(Vec<Job>),
    JobOutput(Result<String, FileWatcherError>),
    Key(KeyEvent),
    MouseClick(usize),
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

const SCANCEL_SIGNALS: &[&str] = &["TERM", "INT", "HUP", "USR1", "USR2", "STOP", "CONT", "KILL"];
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
            _job_watcher: JobWatcherHandle::new(
                sender.clone(),
                Duration::from_secs(slurm_refresh_rate),
                squeue_args,
            ),
            job_list_state: TableState::new(),
            job_sort_field: JobSortField::Id,
            job_sort_direction: SortDirection::Desc,
            job_output: Ok("".to_string()),
            job_output_anchor: ScrollAnchor::Bottom,
            job_output_offset: 0,
            job_output_wrap: false,
            job_output_watcher: FileWatcherHandle::new(
                sender.clone(),
                Duration::from_secs(file_refresh_rate),
            ),
            // sender,
            receiver,
            input_receiver,
            output_file_view: OutputFileView::default(),
            job_list_height: 0,
            job_list_area: Rect::default(),
            job_output_area: Rect::default(),
            pending_input_event: None,
        }
    }
}

mod commands;
mod events;
mod render;
mod sorting;
#[cfg(test)]
mod tests;
