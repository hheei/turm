use super::SlurmClient;
use std::{fmt, process::Command};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SlurmCommand {
    Cancel { id: String, signal: Option<String> },
    SetTimeLimit { id: String, value: String },
    Rename { id: String, value: String },
}

#[derive(Debug)]
pub(crate) struct CommandFailure {
    pub(crate) command: String,
    pub(crate) output: String,
}

impl fmt::Display for CommandFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.command, self.output)
    }
}
impl std::error::Error for CommandFailure {}

impl SlurmClient {
    pub(crate) fn execute(&self, request: SlurmCommand) -> Result<(), CommandFailure> {
        let mut command = match request {
            SlurmCommand::Cancel { id, signal } => {
                let mut command = Command::new(&self.scancel);
                if let Some(signal) = signal {
                    command.arg("--signal").arg(signal);
                }
                command.arg(id);
                command
            }
            SlurmCommand::SetTimeLimit { id, value } => {
                let mut command = Command::new(&self.scontrol);
                command.args([
                    "update",
                    &format!("JobId={id}"),
                    &format!("TimeLimit={value}"),
                ]);
                command
            }
            SlurmCommand::Rename { id, value } => {
                let mut command = Command::new(&self.scontrol);
                command.args(["update", &format!("JobId={id}"), &format!("Name={value}")]);
                command
            }
        };
        self.output(&mut command).map(|_| ())
    }
}
