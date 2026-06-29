use super::commands::{execute_scancel, execute_scontrol_update_timelimit, validated_time_limit};
use super::*;

impl App {
    pub fn run<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> io::Result<()> {
        terminal.draw(|f| self.ui(f))?;

        loop {
            let (should_quit, should_draw) = if let Some(event) = self.pending_input_event.take() {
                self.handle_input_event(event)
            } else {
                select! {
                    recv(self.receiver) -> event => {
                        self.handle(event.unwrap());
                        (false, true)
                    }
                    recv(self.input_receiver) -> input_res => {
                        self.handle_input_event(input_res.unwrap().unwrap())
                    }
                }
            };
            if should_quit {
                return Ok(());
            }

            if should_draw {
                terminal.draw(|f| self.ui(f))?;
            }
            self.flush_pending_clipboard_copy()?;
        }
    }

    fn try_recv_input_event(&mut self) -> Option<Event> {
        if let Some(event) = self.pending_input_event.take() {
            return Some(event);
        }

        loop {
            match self.input_receiver.try_recv() {
                Ok(Ok(event)) => return Some(event),
                Ok(Err(_)) => continue,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return None,
            }
        }
    }

    pub(super) fn handle_input_event(&mut self, event: Event) -> (bool, bool) {
        match event {
            Event::Key(key) => {
                if self.dialog.is_none() && key.code == KeyCode::Char('q') {
                    return (true, false);
                }
                self.handle(AppMessage::Key(key));
                (false, true)
            }
            Event::Paste(_) => (false, false),
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if self.dialog.is_some() {
                        return (false, false);
                    }
                    self.handle(AppMessage::MouseClick {
                        column: mouse.column,
                        row: mouse.row,
                    });
                    (false, true)
                }
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                    if self.dialog.is_some() {
                        return (false, false);
                    }
                    let Some(target) = self.mouse_scroll_target(mouse.column, mouse.row) else {
                        return (false, false);
                    };
                    let direction = mouse_wheel_direction(mouse.kind).unwrap();
                    let mut amount = 1u16;
                    while let Some(next_event) = self.try_recv_input_event() {
                        let should_merge = if let Event::Mouse(next_mouse) = &next_event {
                            mouse_wheel_direction(next_mouse.kind) == Some(direction)
                                && self.mouse_scroll_target(next_mouse.column, next_mouse.row)
                                    == Some(target)
                        } else {
                            false
                        };
                        if should_merge {
                            amount = amount.saturating_add(1);
                        } else {
                            self.pending_input_event = Some(next_event);
                            break;
                        }
                    }
                    self.handle(AppMessage::MouseWheel {
                        target,
                        direction,
                        amount,
                    });
                    (false, true)
                }
                _ => (false, false),
            },
            Event::Resize(_, _) => (false, true),
            _ => (false, false),
        }
    }

    fn mouse_scroll_target(&self, column: u16, row: u16) -> Option<MouseScrollTarget> {
        if rect_contains(self.resource_area, column, row) {
            Some(MouseScrollTarget::Resources)
        } else if rect_contains(self.job_list_area, column, row) {
            Some(MouseScrollTarget::Jobs)
        } else if rect_contains(self.job_output_area, column, row) {
            Some(MouseScrollTarget::Output)
        } else {
            None
        }
    }

    pub(super) fn handle(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::Jobs(jobs) => {
                let selected_id = self.selected_job_id();
                let fallback_index = self.job_list_state.selected();

                self.jobs = jobs;
                self.sort_jobs();
                self.restore_selection_by_job_id(selected_id, fallback_index);
            }
            AppMessage::JobOutput(content) => self.job_output = content,
            AppMessage::Key(key) => {
                if self.dialog.is_some() {
                    let mut close_dialog = false;
                    let mut scancel_request = None;
                    let mut timelimit_request = None;
                    let mut filter_to_apply = None;
                    let mut clipboard_copy = None;
                    let mut command_failure = None;

                    match self.dialog.as_mut().expect("dialog must exist") {
                        Dialog::ConfirmCancelJob { id, signal, .. } => {
                            match cancel_confirmation_action(key) {
                                CancelConfirmationAction::Confirm => {
                                    scancel_request = Some((id.clone(), signal.as_deref()));
                                    close_dialog = true;
                                }
                                CancelConfirmationAction::Cancel => {
                                    close_dialog = true;
                                }
                                CancelConfirmationAction::Ignore => {}
                            }
                        }
                        Dialog::EditTimeLimit { id, input } => match key.code {
                            KeyCode::Enter => {
                                if let Some(time_limit) = validated_time_limit(input) {
                                    timelimit_request = Some((id.clone(), time_limit));
                                    close_dialog = true;
                                }
                            }
                            KeyCode::Esc => {
                                close_dialog = true;
                            }
                            _ => {
                                input.handle_event(&Event::Key(key));
                            }
                        },
                        Dialog::FilterJobs { input } => match key.code {
                            KeyCode::Enter => {
                                filter_to_apply = Some(input.value().to_string());
                                close_dialog = true;
                            }
                            KeyCode::Esc => {
                                close_dialog = true;
                            }
                            KeyCode::Char('u')
                                if key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
                            {
                                *input = Input::new(String::new());
                            }
                            _ => {
                                input.handle_event(&Event::Key(key));
                            }
                        },
                        Dialog::CopyJobOutputDirectory { dir_url, dir_name } => {
                            match copy_job_output_directory_action(key) {
                                CopyJobOutputDirectoryAction::CopyDirUrl => {
                                    clipboard_copy = Some(dir_url.clone());
                                    close_dialog = true;
                                }
                                CopyJobOutputDirectoryAction::CopyDirName => {
                                    clipboard_copy = Some(dir_name.clone());
                                    close_dialog = true;
                                }
                                CopyJobOutputDirectoryAction::Cancel => {
                                    close_dialog = true;
                                }
                                CopyJobOutputDirectoryAction::Ignore => {}
                            }
                        }
                        Dialog::CommandError { .. } => match key.code {
                            KeyCode::Enter | KeyCode::Esc => {
                                close_dialog = true;
                            }
                            _ => {}
                        },
                    };

                    if let Some((id, signal)) = scancel_request {
                        command_failure = execute_scancel(&id, signal).err();
                    }
                    if let Some((id, time_limit)) = timelimit_request {
                        command_failure = execute_scontrol_update_timelimit(&id, &time_limit).err();
                    }
                    if let Some(filter) = filter_to_apply {
                        self.apply_job_filter(&filter);
                    }
                    if let Some(copy_value) = clipboard_copy {
                        self.pending_clipboard_copy = Some(copy_value);
                    }
                    if let Some(CommandFailure { command, output }) = command_failure {
                        self.dialog = Some(Dialog::CommandError { command, output });
                    } else if close_dialog {
                        self.dialog = None;
                    }
                } else {
                    match key.code {
                        KeyCode::Char(']') => self.focus_next_panel(),
                        KeyCode::Char('[') => self.focus_previous_panel(),
                        KeyCode::Char('k') | KeyCode::Up => match self.focus {
                            Focus::Resources => self.select_previous_resource(),
                            Focus::Jobs => self.select_previous_job(),
                            Focus::Details => {}
                            Focus::Log => self.scroll_job_output_up_by(1),
                        },
                        KeyCode::Char('j') | KeyCode::Down => match self.focus {
                            Focus::Resources => self.select_next_resource(),
                            Focus::Jobs => self.select_next_job(),
                            Focus::Details => {}
                            Focus::Log => self.scroll_job_output_down_by(1),
                        },
                        KeyCode::Char('g') => match self.focus {
                            Focus::Resources => self.select_first_resource(),
                            Focus::Jobs => self.select_first_job(),
                            Focus::Details => {}
                            Focus::Log => self.scroll_job_output_to_top(),
                        },
                        KeyCode::Char('G') => match self.focus {
                            Focus::Resources => self.select_last_resource(),
                            Focus::Jobs => self.select_last_job(),
                            Focus::Details => {}
                            Focus::Log => self.scroll_job_output_to_bottom(),
                        },
                        KeyCode::Char('f') if matches!(self.focus, Focus::Jobs) => {
                            self.dialog = Some(Dialog::FilterJobs {
                                input: Input::new(self.active_filter.clone()),
                            });
                        }
                        KeyCode::Char('s') if matches!(self.focus, Focus::Jobs) => {
                            self.update_job_sort(JobSortField::State);
                        }
                        KeyCode::Char('p') if matches!(self.focus, Focus::Jobs) => {
                            self.update_job_sort(JobSortField::Partition);
                        }
                        KeyCode::Char('i') if matches!(self.focus, Focus::Jobs) => {
                            self.update_job_sort(JobSortField::Id);
                        }
                        KeyCode::Char('n') if matches!(self.focus, Focus::Jobs) => {
                            self.update_job_sort(JobSortField::Name);
                        }
                        KeyCode::Char('u') => {
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                match self.focus {
                                    Focus::Resources => {}
                                    Focus::Jobs => self.scroll_jobs_half_page_up(),
                                    Focus::Details => {}
                                    Focus::Log => self.scroll_job_output_half_page_up(),
                                }
                            } else if matches!(self.focus, Focus::Jobs) {
                                self.update_job_sort(JobSortField::User);
                            }
                        }
                        KeyCode::Char('d') => {
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                self.dialog = self.cancel_confirmation_dialog();
                            }
                        }
                        KeyCode::PageDown if matches!(self.focus, Focus::Log) => {
                            let delta = if key.modifiers.intersects(
                                crossterm::event::KeyModifiers::SHIFT
                                    | crossterm::event::KeyModifiers::CONTROL
                                    | crossterm::event::KeyModifiers::ALT,
                            ) {
                                50
                            } else {
                                1
                            };
                            self.scroll_job_output_down_by(delta);
                        }
                        KeyCode::PageUp if matches!(self.focus, Focus::Log) => {
                            let delta = if key.modifiers.intersects(
                                crossterm::event::KeyModifiers::SHIFT
                                    | crossterm::event::KeyModifiers::CONTROL
                                    | crossterm::event::KeyModifiers::ALT,
                            ) {
                                50
                            } else {
                                1
                            };
                            self.scroll_job_output_up_by(delta);
                        }
                        KeyCode::Home if matches!(self.focus, Focus::Log) => {
                            self.scroll_job_output_to_top();
                        }
                        KeyCode::End if matches!(self.focus, Focus::Log) => {
                            self.scroll_job_output_to_bottom();
                        }
                        KeyCode::Char('c') if matches!(self.focus, Focus::Jobs) => {
                            self.dialog = self.copy_job_output_directory_dialog();
                        }
                        KeyCode::Char('C') if matches!(self.focus, Focus::Jobs) => {}
                        KeyCode::Char('t') if matches!(self.focus, Focus::Jobs) => {
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                if let Some(job) = self.selected_job() {
                                    self.dialog = Some(Dialog::EditTimeLimit {
                                        id: job.id(),
                                        input: Input::new(job.time_limit.clone()),
                                    });
                                }
                            } else {
                                self.update_job_sort(JobSortField::Time);
                            }
                        }
                        KeyCode::Char('o') if matches!(self.focus, Focus::Log) => {
                            self.output_file_view = match self.output_file_view {
                                OutputFileView::Stdout => OutputFileView::Stderr,
                                OutputFileView::Stderr => OutputFileView::Stdout,
                            };
                        }
                        KeyCode::Char('w') if matches!(self.focus, Focus::Log) => {
                            self.job_output_wrap = !self.job_output_wrap;
                        }
                        _ => {}
                    };
                }
            }
            AppMessage::MouseClick { column, row } => {
                if self.dialog.is_none() {
                    if let Some(index) = self.job_index_at(column, row) {
                        self.focus = Focus::Jobs;
                        if index < self.visible_job_indices().len() {
                            self.job_list_state.select(Some(index));
                        }
                    } else if rect_contains(self.resource_area, column, row) {
                        self.focus = Focus::Resources;
                    } else if rect_contains(self.job_details_area, column, row) {
                        self.focus = Focus::Details;
                    } else if rect_contains(self.job_output_area, column, row) {
                        self.focus = Focus::Log;
                    }
                }
            }
            AppMessage::MouseWheel {
                target,
                direction,
                amount,
            } => {
                if self.dialog.is_none() {
                    match target {
                        MouseScrollTarget::Resources => match direction {
                            MouseWheelDirection::Up => {
                                self.resource_table_state.scroll_up_by(amount)
                            }
                            MouseWheelDirection::Down => {
                                self.resource_table_state.scroll_down_by(amount)
                            }
                        },
                        MouseScrollTarget::Jobs => match direction {
                            MouseWheelDirection::Up => self.job_list_state.scroll_up_by(amount),
                            MouseWheelDirection::Down => self.job_list_state.scroll_down_by(amount),
                        },
                        MouseScrollTarget::Output => match direction {
                            MouseWheelDirection::Up => self.scroll_job_output_up_by(amount),
                            MouseWheelDirection::Down => self.scroll_job_output_down_by(amount),
                        },
                    }
                }
            }
        }

        self.job_output_watcher
            .set_file_path(
                self.selected_job()
                    .and_then(|job| match self.output_file_view {
                        OutputFileView::Stdout => job.stdout.clone(),
                        OutputFileView::Stderr => job.stderr.clone(),
                    }),
            );
    }

    pub(super) fn selected_job(&self) -> Option<&Job> {
        let visible_job_indices = self.visible_job_indices();
        self.job_list_state
            .selected()
            .and_then(|index| visible_job_indices.get(index).copied())
            .and_then(|index| self.jobs.get(index))
    }

    pub(super) fn selected_job_id(&self) -> Option<String> {
        self.selected_job().map(Job::id)
    }

    fn flush_pending_clipboard_copy(&mut self) -> io::Result<()> {
        if let Some(value) = self.pending_clipboard_copy.take() {
            write_osc52_clipboard(&value)?;
        }

        Ok(())
    }

    fn copy_job_output_directory_dialog(&self) -> Option<Dialog> {
        let directory = self
            .selected_job()
            .and_then(|job| preferred_output_path(job, self.output_file_view))
            .and_then(|path| path.parent())?;
        let (dir_url, dir_name) = copy_job_output_directory_value(directory)?;

        Some(Dialog::CopyJobOutputDirectory { dir_url, dir_name })
    }

    fn cancel_confirmation_dialog(&self) -> Option<Dialog> {
        let job = self.selected_job()?;
        Some(Dialog::ConfirmCancelJob {
            id: job.id(),
            name: job.name.clone(),
            details: selected_job_cancel_details(job),
            signal: None,
        })
    }

    fn focus_next_panel(&mut self) {
        self.focus = match self.focus {
            Focus::Resources => Focus::Jobs,
            Focus::Jobs => Focus::Details,
            Focus::Details => Focus::Log,
            Focus::Log => Focus::Resources,
        };
    }

    fn focus_previous_panel(&mut self) {
        self.focus = match self.focus {
            Focus::Resources => Focus::Log,
            Focus::Jobs => Focus::Resources,
            Focus::Details => Focus::Jobs,
            Focus::Log => Focus::Details,
        };
    }

    fn select_next_job(&mut self) {
        if !self.visible_job_indices().is_empty() {
            self.job_list_state.select_next();
        }
    }

    fn select_previous_job(&mut self) {
        if !self.visible_job_indices().is_empty() {
            self.job_list_state.select_previous();
        }
    }

    fn select_first_job(&mut self) {
        if !self.visible_job_indices().is_empty() {
            self.job_list_state.select_first();
        }
    }

    fn select_last_job(&mut self) {
        if !self.visible_job_indices().is_empty() {
            self.job_list_state.select_last();
        }
    }

    fn select_next_resource(&mut self) {
        if !self.resources.is_empty() {
            self.resource_table_state.select_next();
        }
    }

    fn select_previous_resource(&mut self) {
        if !self.resources.is_empty() {
            self.resource_table_state.select_previous();
        }
    }

    fn select_first_resource(&mut self) {
        if !self.resources.is_empty() {
            self.resource_table_state.select_first();
        }
    }

    fn select_last_resource(&mut self) {
        if !self.resources.is_empty() {
            self.resource_table_state.select_last();
        }
    }

    pub(super) fn scroll_jobs_half_page_up(&mut self) {
        if !self.visible_job_indices().is_empty() {
            self.job_list_state.scroll_up_by(self.job_list_height / 2);
        }
    }

    pub(super) fn job_list_rows_area(&self) -> Rect {
        Rect::new(
            self.job_list_area.x.saturating_add(1),
            self.job_list_area.y.saturating_add(2),
            self.job_list_area.width.saturating_sub(2),
            self.job_list_height,
        )
    }

    fn job_index_at(&self, column: u16, row: u16) -> Option<usize> {
        let visible_job_indices = self.visible_job_indices();
        if visible_job_indices.is_empty() {
            return None;
        }
        let rows_area = self.job_list_rows_area();
        if !rect_contains(rows_area, column, row) {
            return None;
        }

        let row_in_list = (row - rows_area.y) as usize;
        let index = self.job_list_state.offset().saturating_add(row_in_list);
        (index < visible_job_indices.len()).then_some(index)
    }

    fn scroll_job_output_half_page_up(&mut self) {
        self.scroll_job_output_up_by(self.job_output_page_step());
    }

    fn scroll_job_output_to_top(&mut self) {
        self.job_output_offset = 0;
        self.job_output_anchor = ScrollAnchor::Top;
    }

    fn scroll_job_output_to_bottom(&mut self) {
        self.job_output_offset = 0;
        self.job_output_anchor = ScrollAnchor::Bottom;
    }

    fn job_output_page_step(&self) -> u16 {
        self.job_output_area
            .height
            .saturating_sub(2)
            .saturating_div(2)
            .max(1)
    }

    fn scroll_job_output_down_by(&mut self, delta: u16) {
        match self.job_output_anchor {
            ScrollAnchor::Top => {
                self.job_output_offset = self.job_output_offset.saturating_add(delta)
            }
            ScrollAnchor::Bottom => {
                self.job_output_offset = self.job_output_offset.saturating_sub(delta)
            }
        }
    }

    fn scroll_job_output_up_by(&mut self, delta: u16) {
        match self.job_output_anchor {
            ScrollAnchor::Top => {
                self.job_output_offset = self.job_output_offset.saturating_sub(delta)
            }
            ScrollAnchor::Bottom => {
                self.job_output_offset = self.job_output_offset.saturating_add(delta)
            }
        }
    }
}

fn rect_contains(rect: Rect, column: u16, row: u16) -> bool {
    column >= rect.x
        && column < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

fn mouse_wheel_direction(kind: MouseEventKind) -> Option<MouseWheelDirection> {
    match kind {
        MouseEventKind::ScrollUp => Some(MouseWheelDirection::Up),
        MouseEventKind::ScrollDown => Some(MouseWheelDirection::Down),
        _ => None,
    }
}

fn selected_job_cancel_details(job: &Job) -> Vec<String> {
    let mut details = Vec::new();
    let location = preferred_output_path(job, OutputFileView::Stdout)
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

fn preferred_output_path(job: &Job, view: OutputFileView) -> Option<&PathBuf> {
    match view {
        OutputFileView::Stdout => job.stdout.as_ref().or(job.stderr.as_ref()),
        OutputFileView::Stderr => job.stderr.as_ref().or(job.stdout.as_ref()),
    }
}

fn copy_job_output_directory_value(path: &std::path::Path) -> Option<(String, String)> {
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

fn write_osc52_clipboard(value: &str) -> io::Result<()> {
    use std::io::Write;

    let mut stdout = io::stdout();
    write!(stdout, "\x1b]52;c;{}\x07", base64_encode(value.as_bytes()))?;
    stdout.flush()
}

fn base64_encode(bytes: &[u8]) -> String {
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
pub(super) enum CancelConfirmationAction {
    Confirm,
    Cancel,
    Ignore,
}

pub(super) fn cancel_confirmation_action(key: KeyEvent) -> CancelConfirmationAction {
    match key.code {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            CancelConfirmationAction::Confirm
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => CancelConfirmationAction::Cancel,
        _ => CancelConfirmationAction::Ignore,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CopyJobOutputDirectoryAction {
    CopyDirUrl,
    CopyDirName,
    Cancel,
    Ignore,
}

pub(super) fn copy_job_output_directory_action(key: KeyEvent) -> CopyJobOutputDirectoryAction {
    match key.code {
        KeyCode::Char('c') => CopyJobOutputDirectoryAction::CopyDirUrl,
        KeyCode::Char('d') => CopyJobOutputDirectoryAction::CopyDirName,
        KeyCode::Esc => CopyJobOutputDirectoryAction::Cancel,
        _ => CopyJobOutputDirectoryAction::Ignore,
    }
}
