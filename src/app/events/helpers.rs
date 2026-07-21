use super::*;

pub(in crate::app) fn rect_contains(rect: Rect, column: u16, row: u16) -> bool {
    column >= rect.x
        && column < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

pub(in crate::app) fn mouse_wheel_direction(kind: MouseEventKind) -> Option<MouseWheelDirection> {
    match kind {
        MouseEventKind::ScrollUp => Some(MouseWheelDirection::Up),
        MouseEventKind::ScrollDown => Some(MouseWheelDirection::Down),
        _ => None,
    }
}

pub(in crate::app) fn selected_job_cancel_details(job: &Job) -> Vec<String> {
    let mut details = Vec::new();
    let location = preferred_output_path(job, OutputPanelMode::Stdout)
        .and_then(|path| path.parent())
        .map(|path| path.to_string_lossy().to_string())
        .filter(|value| !value.is_empty());

    if let Some(location) = location {
        details.push(location);
    } else if !job.command.trim().is_empty() {
        details.push(job.command.clone());
    }

    if !job.user.trim().is_empty() || !job.partition.trim().is_empty() {
        details.push(format!(
            "{}{}{}",
            if job.user.trim().is_empty() {
                ""
            } else {
                &job.user
            },
            if job.user.trim().is_empty() || job.partition.trim().is_empty() {
                ""
            } else {
                " • "
            },
            if job.partition.trim().is_empty() {
                ""
            } else {
                &job.partition
            }
        ));
    }

    details
}

pub(in crate::app) fn preferred_output_path(job: &Job, mode: OutputPanelMode) -> Option<&PathBuf> {
    match mode {
        OutputPanelMode::Stdout => job.stdout.as_ref().or(job.stderr.as_ref()),
        OutputPanelMode::Stderr => job.stderr.as_ref().or(job.stdout.as_ref()),
        OutputPanelMode::Workdir | OutputPanelMode::Collapsed => {
            job.stdout.as_ref().or(job.stderr.as_ref())
        }
    }
}

pub(in crate::app) fn output_directory_for_mode(
    job: &Job,
    mode: OutputPanelMode,
) -> Option<PathBuf> {
    match mode {
        OutputPanelMode::Workdir => App::derive_workdir_path(job),
        OutputPanelMode::Stdout | OutputPanelMode::Stderr | OutputPanelMode::Collapsed => {
            preferred_output_path(job, mode).and_then(|path| path.parent().map(PathBuf::from))
        }
    }
}

pub(in crate::app) fn watched_output_path(job: &Job, mode: OutputPanelMode) -> Option<PathBuf> {
    match mode {
        OutputPanelMode::Stdout => job.stdout.clone(),
        OutputPanelMode::Stderr => job.stderr.clone(),
        OutputPanelMode::Workdir | OutputPanelMode::Collapsed => None,
    }
}

pub(in crate::app) fn load_workdir_entries(path: &Path) -> io::Result<Vec<WorkdirEntry>> {
    let mut entries = fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            let kind = if file_type.is_symlink() {
                WorkdirEntryKind::Symlink
            } else if file_type.is_dir() {
                WorkdirEntryKind::Directory
            } else {
                WorkdirEntryKind::File
            };
            Some(WorkdirEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                path: entry.path(),
                kind,
            })
        })
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| match (left.kind, right.kind) {
        (WorkdirEntryKind::Directory, WorkdirEntryKind::File) => Ordering::Less,
        (WorkdirEntryKind::File, WorkdirEntryKind::Directory) => Ordering::Greater,
        (WorkdirEntryKind::Symlink, WorkdirEntryKind::Directory) => Ordering::Greater,
        (WorkdirEntryKind::Directory, WorkdirEntryKind::Symlink) => Ordering::Less,
        _ => left.name.cmp(&right.name),
    });
    Ok(entries)
}

pub(in crate::app) fn command_parent_path(command: &str) -> Option<PathBuf> {
    command
        .split_whitespace()
        .map(|token| token.trim_matches(|c| c == '"' || c == '\''))
        .find_map(|token| {
            if token.is_empty() || (!token.contains('/') && !token.starts_with('.')) {
                return None;
            }
            Path::new(token).parent().map(PathBuf::from)
        })
}

pub(in crate::app) fn workdir_entry_label(entry: &WorkdirEntry) -> String {
    match entry.kind {
        WorkdirEntryKind::Directory => format!(" {}/", entry.name),
        WorkdirEntryKind::File => format!(" {}", entry.name),
        WorkdirEntryKind::Symlink => format!(" {}", entry.name),
    }
}

pub(in crate::app) fn max_line_chars(s: &str) -> usize {
    let s = s.rsplit_once(['\r', '\n']).map_or(s, |(prefix, _)| prefix);
    s.lines()
        .flat_map(|line| line.split('\r'))
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
}
pub(in crate::app) fn copy_job_output_directory_value(path: &Path) -> Option<(String, String)> {
    let dir_path = path.to_string_lossy().to_string();
    let dir_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| dir_path.clone());

    if dir_path.is_empty() || dir_name.is_empty() {
        None
    } else {
        Some((dir_path, dir_name))
    }
}

pub(in crate::app) fn write_osc52_clipboard(value: &str) -> io::Result<()> {
    use std::io::Write;

    let mut stdout = io::stdout();
    write!(stdout, "\x1b]52;c;{}\x07", base64_encode(value.as_bytes()))?;
    stdout.flush()
}

pub(in crate::app) fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut index = 0;
    while index < bytes.len() {
        let remaining = bytes.len() - index;
        let first = bytes[index];
        let second = if remaining > 1 { bytes[index + 1] } else { 0 };
        let third = if remaining > 2 { bytes[index + 2] } else { 0 };
        let chunk = ((first as u32) << 16) | ((second as u32) << 8) | third as u32;

        encoded.push(ALPHABET[((chunk >> 18) & 0x3F) as usize] as char);
        encoded.push(ALPHABET[((chunk >> 12) & 0x3F) as usize] as char);
        encoded.push(if remaining > 1 {
            ALPHABET[((chunk >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        encoded.push(if remaining > 2 {
            ALPHABET[(chunk & 0x3F) as usize] as char
        } else {
            '='
        });
        index += 3;
    }

    encoded
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelConfirmationAction {
    Confirm,
    Cancel,
    Select(ConfirmCancelChoice),
    Ignore,
}

pub(in crate::app) fn cancel_confirmation_action(
    key: KeyEvent,
    selected: ConfirmCancelChoice,
) -> CancelConfirmationAction {
    match key.code {
        KeyCode::Enter => match selected {
            ConfirmCancelChoice::Yes => CancelConfirmationAction::Confirm,
            ConfirmCancelChoice::No => CancelConfirmationAction::Cancel,
        },
        KeyCode::Left | KeyCode::Char('h') => {
            CancelConfirmationAction::Select(ConfirmCancelChoice::No)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            CancelConfirmationAction::Select(ConfirmCancelChoice::Yes)
        }
        KeyCode::Char('y') | KeyCode::Char('Y') => CancelConfirmationAction::Confirm,
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => CancelConfirmationAction::Cancel,
        _ => CancelConfirmationAction::Ignore,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum CopyJobOutputDirectoryAction {
    CopyDirUrl,
    CopyDirName,
    Cancel,
    Ignore,
}

pub(in crate::app) fn copy_job_output_directory_action(
    key: KeyEvent,
) -> CopyJobOutputDirectoryAction {
    match key.code {
        KeyCode::Char('d') => CopyJobOutputDirectoryAction::CopyDirUrl,
        KeyCode::Char('c') => CopyJobOutputDirectoryAction::CopyDirName,
        KeyCode::Esc => CopyJobOutputDirectoryAction::Cancel,
        _ => CopyJobOutputDirectoryAction::Ignore,
    }
}
