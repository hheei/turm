pub use super::events::CancelConfirmationAction;
use super::events::{OutputLayout, cancel_confirmation_action, watched_output_path};
use super::render::{OUTPUT_HORIZONTAL_SCROLLBAR_THUMB, VERTICAL_SCROLLBAR_THUMB, chunked_string};
use super::*;
use crossbeam::channel::unbounded;
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use std::{path::Path, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestJobSortField {
    State,
    Partition,
    Id,
    Name,
    User,
    Time,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestWorkdirEntryKind {
    Directory,
    File,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestWorkdirEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: TestWorkdirEntryKind,
}

pub type WorkdirEntrySnapshot = TestWorkdirEntry;
pub type WorkdirEntryKindSnapshot = TestWorkdirEntryKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutSnapshot {
    pub viewport: Rect,
    pub show_vertical: bool,
    pub show_horizontal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSnapshot {
    pub focus: Focus,
    pub has_dialog: bool,
    pub selected_job_index: Option<usize>,
    pub active_filter: String,
    pub sort_field: TestJobSortField,
    pub sort_direction: TestSortDirection,
    pub output_mode: OutputPanelMode,
    pub details_visible: bool,
    pub output_anchor: ScrollAnchor,
    pub output_offset: u16,
    pub output_scroll_x: u16,
    pub output_wrap: bool,
    pub workdir_selected: Option<usize>,
    pub workdir_offset: usize,
    pub pending_clipboard_copy: Option<String>,
    pub job_list_area: Rect,
    pub job_details_area: Rect,
    pub job_output_area: Rect,
    pub resource_area: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceSnapshot {
    pub partition: String,
    pub total_nodes: u32,
    pub running_nodes: u32,
    pub group_used_nodes: u32,
    pub available_nodes: u32,
}

pub struct AppDriver {
    app: App,
}

mod driver;
mod fixtures;
mod jobs;
mod resources;

pub use fixtures::*;
pub use jobs::*;
pub use resources::*;
