#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Resources,
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
        selected: ConfirmCancelChoice,
    },
    EditTimeLimit {
        id: String,
        input: Input,
    },
    EditJobName {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmCancelChoice {
    No,
    Yes,
}

pub(in crate::app) struct CommandFailure {
    pub(in crate::app) command: String,
    pub(in crate::app) output: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAnchor {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputPanelMode {
    Stdout,
    Stderr,
    #[default]
    Workdir,
    Collapsed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppExit {
    OpenEditor(PathBuf),
    ChangeDirectory(PathBuf),
}

impl OutputPanelMode {
    pub fn next(self) -> Self {
        match self {
            Self::Stdout => Self::Stderr,
            Self::Stderr => Self::Collapsed,
            Self::Workdir => Self::Stdout,
            Self::Collapsed => Self::Workdir,
        }
    }

    pub fn is_collapsed(self) -> bool {
        self == Self::Collapsed
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Stdout => "Stdout",
            Self::Stderr => "Stderr",
            Self::Workdir => "Workdir",
            Self::Collapsed => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum WorkdirEntryKind {
    Directory,
    File,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct WorkdirEntry {
    pub(in crate::app) name: String,
    pub(in crate::app) path: PathBuf,
    pub(in crate::app) kind: WorkdirEntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum JobSortField {
    State,
    Partition,
    Id,
    Name,
    User,
    Time,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum SortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Copy)]
pub(in crate::app) enum JobFilterField {
    Job,
    Id,
    Name,
    User,
    Partition,
    State,
    Time,
}

pub(in crate::app) enum JobFilter {
    None,
    FreeText(String),
    Field(JobFilterField, String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum SelectionArea {
    Resources,
    Jobs,
    Details,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) struct MouseSelection {
    pub(in crate::app) area: SelectionArea,
    pub(in crate::app) bounds: Rect,
    pub(in crate::app) start: Position,
    pub(in crate::app) end: Position,
    pub(in crate::app) dragged: bool,
}

#[derive(Clone, Copy)]
pub(in crate::app) struct DetailsSelectionRow {
    pub(in crate::app) y: u16,
    pub(in crate::app) group: usize,
    pub(in crate::app) left: u16,
    pub(in crate::app) value_x: u16,
    pub(in crate::app) right: u16,
}

impl MouseSelection {
    pub(in crate::app) fn row_bounds(self, row: u16) -> Option<(u16, u16)> {
        let (start, end) = if (self.start.y, self.start.x) <= (self.end.y, self.end.x) {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        };
        if !(start.y..=end.y).contains(&row) {
            return None;
        }

        let left = self.bounds.x;
        let right = self.bounds.right().saturating_sub(1);
        Some(if start.y == end.y {
            (start.x, end.x)
        } else if row == start.y {
            (left, right)
        } else if row == end.y {
            (left, end.x)
        } else {
            (left, right)
        })
    }
}

pub struct App {
    pub(in crate::app) focus: Focus,
    pub(in crate::app) dialog: Option<Dialog>,
    pub(in crate::app) jobs: Vec<Job>,
    pub(in crate::app) active_filter: String,
    pub(in crate::app) job_list_state: TableState,
    pub(in crate::app) job_sort_field: JobSortField,
    pub(in crate::app) job_sort_direction: SortDirection,
    pub(in crate::app) job_output: Result<String, FileWatcherError>,
    pub(in crate::app) job_output_anchor: ScrollAnchor,
    pub(in crate::app) job_output_offset: u16,
    pub(in crate::app) output_scroll_x: u16,
    pub(in crate::app) job_output_wrap: bool,
    pub(in crate::app) workdir_path: Option<PathBuf>,
    pub(in crate::app) workdir_entries: Vec<WorkdirEntry>,
    pub(in crate::app) workdir_error: Option<String>,
    pub(in crate::app) workdir_selected: Option<usize>,
    pub(in crate::app) workdir_offset: usize,
    pub(in crate::app) _job_watcher: JobWatcherHandle,
    pub(in crate::app) _resource_watcher: ResourceWatcherHandle,
    pub(in crate::app) job_output_watcher: FileWatcherHandle,
    // sender: Sender<AppMessage>,
    pub(in crate::app) receiver: Receiver<AppMessage>,
    pub(in crate::app) input_receiver: Receiver<std::io::Result<Event>>,
    pub(in crate::app) output_panel_mode: OutputPanelMode,
    pub(in crate::app) output_can_expand: bool,
    pub(in crate::app) details_visible: bool,
    pub(in crate::app) job_list_height: u16,
    pub(in crate::app) job_list_area: Rect,
    pub(in crate::app) job_details_area: Rect,
    pub(in crate::app) job_output_area: Rect,
    pub(in crate::app) pending_input_event: Option<Event>,
    pub(in crate::app) pending_clipboard_copy: Option<String>,
    pub(in crate::app) clipboard_notice_until: Option<Instant>,
    pub(in crate::app) pending_exit: Option<AppExit>,
    pub(in crate::app) mouse_selection: Option<MouseSelection>,
    pub(in crate::app) details_selection_rows: Vec<DetailsSelectionRow>,
    pub(in crate::app) screen_buffer: Option<ratatui::buffer::Buffer>,
    pub(in crate::app) resource_table_state: TableState,
    pub(in crate::app) resource_list_height: u16,
    pub(in crate::app) resource_area: Rect,
    pub(in crate::app) resources: Vec<PartitionResources>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartitionResources {
    pub(crate) partition: String,
    pub(crate) total_nodes: u32,
    pub(crate) running_nodes: u32,
    pub(crate) group_used_nodes: u32,
    pub(crate) available_nodes: u32,
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
    pub workdir: Option<PathBuf>,
    pub command: String,
}

impl Job {
    pub(in crate::app) fn id(&self) -> String {
        match self.array_step.as_ref() {
            Some(array_step) => format!("{}_{}", self.array_id, array_step),
            None => self.job_id.clone(),
        }
    }
}
use super::*;
