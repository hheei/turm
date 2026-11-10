use crossbeam::{
    channel::{Receiver, TryRecvError, after, never, unbounded},
    select,
};
use itertools::Either;
use std::{
    cmp::{Ordering, min},
    iter::once,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::file_watcher::{FileWatcherError, FileWatcherHandle};
use crate::slurm::{CommandFailure, JobSnapshots, SlurmCommand, SlurmEvent, SlurmHandle};

use crossterm::event::{Event, KeyCode, KeyEvent, MouseButton, MouseEventKind};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Padding, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState, Wrap,
    },
};
use tui_input::{Input, backend::crossterm::EventHandler};

mod commands;
mod core;
mod events;
mod render;
mod sorting;
#[doc(hidden)]
pub mod test_support;

pub(crate) use core::PartitionResources;
pub use core::{
    App, AppExit, AppMessage, ConfirmCancelChoice, Dialog, Focus, Job, OutputPanelMode,
    ScrollAnchor,
};
use core::{
    DIALOG_WIDTH, DetailsSelectionRow, JobSortField, MouseScrollTarget, MouseSelection,
    MouseWheelDirection, SelectionArea, SortDirection, WorkdirEntry, WorkdirEntryKind,
};
