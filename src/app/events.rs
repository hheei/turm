use super::commands::{
    execute_scancel, execute_scontrol_update_job_name, execute_scontrol_update_timelimit,
    validated_time_limit,
};
use super::render::job_output_line_count;
use super::*;
use std::{
    cmp::Ordering,
    fs, io,
    path::{Path, PathBuf},
};

pub(super) struct OutputLayout {
    pub(super) viewport: Rect,
    pub(super) show_vertical: bool,
    pub(super) show_horizontal: bool,
}

mod helpers;
mod messages;
mod navigation;
mod output;
mod runtime;
mod selection;
mod workdir;

pub use helpers::CancelConfirmationAction;
use helpers::*;
pub(super) use helpers::{cancel_confirmation_action, watched_output_path, workdir_entry_label};
