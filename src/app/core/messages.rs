#[allow(private_interfaces)]
pub enum AppMessage {
    Jobs(Vec<Job>),
    JobOutput(Result<String, FileWatcherError>),
    ResourcesUpdated(Vec<PartitionResources>),
    ResourceWatcherError(#[allow(dead_code)] String),
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
pub(in crate::app) enum MouseWheelDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum MouseScrollTarget {
    Resources,
    Jobs,
    Output,
}

pub(in crate::app) const DIALOG_WIDTH: u16 = 80;
use super::*;
