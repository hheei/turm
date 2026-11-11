use std::{path::PathBuf, process::Command, sync::OnceLock};

use super::CommandFailure;

/// Single process boundary. Explicit paths keep fixtures isolated from PATH
/// and provide a place for process timeouts and diagnostics in a later phase.
pub(crate) struct SlurmClient {
    pub(crate) squeue: PathBuf,
    pub(crate) sacct: PathBuf,
    pub(crate) sinfo: PathBuf,
    pub(crate) scancel: PathBuf,
    pub(crate) scontrol: PathBuf,
    group: OnceLock<Option<String>>,
    history_format: OnceLock<Result<super::jobs::HistoryFormat, String>>,
}

impl Default for SlurmClient {
    fn default() -> Self {
        Self::in_directory("")
    }
}

impl SlurmClient {
    pub(crate) fn in_directory(directory: impl Into<PathBuf>) -> Self {
        let directory = directory.into();
        Self {
            squeue: directory.join("squeue"),
            sacct: directory.join("sacct"),
            sinfo: directory.join("sinfo"),
            scancel: directory.join("scancel"),
            scontrol: directory.join("scontrol"),
            group: OnceLock::new(),
            history_format: OnceLock::new(),
        }
    }

    pub(super) fn history_format(&self) -> Result<&super::jobs::HistoryFormat, String> {
        self.history_format
            .get_or_init(|| super::jobs::HistoryFormat::detect(self))
            .as_ref()
            .map_err(|error| error.clone())
    }

    pub(crate) fn primary_group(&self) -> Result<&str, String> {
        self.group
            .get_or_init(crate::squeue_args::primary_group)
            .as_deref()
            .ok_or_else(|| "failed to determine primary group".into())
    }

    pub(crate) fn output(&self, command: &mut Command) -> Result<String, CommandFailure> {
        let label = std::iter::once(command.get_program())
            .chain(command.get_args())
            .map(|part| part.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        let output = command.output().map_err(|error| CommandFailure {
            command: label.clone(),
            output: error.to_string(),
        })?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        let mut details = vec![format!(
            "Exit code: {}",
            output
                .status
                .code()
                .map_or("N/A".into(), |code| code.to_string())
        )];
        for (name, bytes) in [("stdout", &output.stdout), ("stderr", &output.stderr)] {
            let text = String::from_utf8_lossy(bytes);
            if !text.trim_end().is_empty() {
                details.push(format!("{name}:\n{}", text.trim_end()));
            }
        }
        if details.len() == 1 {
            details.push("No output.".into());
        }
        Err(CommandFailure {
            command: label,
            output: details.join("\n\n"),
        })
    }
}
