use super::*;
use super::commands::{
    execute_scancel,
    execute_scontrol_update_timelimit,
    validated_time_limit,
};

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

    fn handle_input_event(&mut self, event: Event) -> (bool, bool) {
        match event {
            Event::Key(key) => {
                if key.code == KeyCode::Char('q') {
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
                    if let Some(index) = self.job_index_at(mouse.column, mouse.row) {
                        if self.job_list_state.selected() != Some(index) {
                            self.handle(AppMessage::MouseClick(index));
                            return (false, true);
                        }
                    }
                    (false, false)
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
        if rect_contains(self.job_list_area, column, row) {
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
                    let mut command_failure = None;

                    match self.dialog.as_mut().expect("dialog must exist") {
                        Dialog::ConfirmCancelJob(id) => match key.code {
                            KeyCode::Enter | KeyCode::Char('y') => {
                                scancel_request = Some((id.clone(), None));
                                close_dialog = true;
                            }
                            KeyCode::Esc => {
                                close_dialog = true;
                            }
                            _ => {}
                        },
                        Dialog::SelectCancelSignal {
                            id,
                            selected_signal,
                        } => match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                *selected_signal = selected_signal.saturating_sub(1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                *selected_signal = min(
                                    selected_signal.saturating_add(1),
                                    SCANCEL_SIGNALS.len().saturating_sub(1),
                                );
                            }
                            KeyCode::Enter => {
                                scancel_request =
                                    Some((id.clone(), Some(SCANCEL_SIGNALS[*selected_signal])));
                                close_dialog = true;
                            }
                            KeyCode::Esc => {
                                close_dialog = true;
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() => {
                                if let Some(index) = signal_index_for_digit(c) {
                                    if index < SCANCEL_SIGNALS.len() {
                                        *selected_signal = index;
                                    }
                                }
                            }
                            _ => {}
                        },
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
                    if let Some(CommandFailure { command, output }) = command_failure {
                        self.dialog = Some(Dialog::CommandError { command, output });
                    } else if close_dialog {
                        self.dialog = None;
                    }
                } else {
                    match key.code {
                        KeyCode::Char('h') | KeyCode::Left => self.focus_previous_panel(),
                        KeyCode::Char('l') | KeyCode::Right => self.focus_next_panel(),
                        KeyCode::Char('k') | KeyCode::Up => match self.focus {
                            Focus::Jobs => self.select_previous_job(),
                        },
                        KeyCode::Char('j') | KeyCode::Down => match self.focus {
                            Focus::Jobs => self.select_next_job(),
                        },
                        KeyCode::Char('g') => match self.focus {
                            Focus::Jobs => self.select_first_job(),
                        },
                        KeyCode::Char('G') => match self.focus {
                            Focus::Jobs => self.select_last_job(),
                        },
                        KeyCode::Char('s') => self.update_job_sort(JobSortField::State),
                        KeyCode::Char('p') => self.update_job_sort(JobSortField::Partition),
                        KeyCode::Char('i') => self.update_job_sort(JobSortField::Id),
                        KeyCode::Char('n') => self.update_job_sort(JobSortField::Name),
                        KeyCode::Char('u') => match self.focus {
                            Focus::Jobs => {
                                if key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.scroll_jobs_half_page_up()
                                } else {
                                    self.update_job_sort(JobSortField::User)
                                }
                            }
                        },
                        KeyCode::Char('d') => match self.focus {
                            Focus::Jobs => {
                                if key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.scroll_jobs_half_page_down()
                                }
                            }
                        },
                        KeyCode::PageDown => {
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
                        KeyCode::PageUp => {
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
                        KeyCode::Home => {
                            self.job_output_offset = 0;
                            self.job_output_anchor = ScrollAnchor::Top;
                        }
                        KeyCode::End => {
                            self.job_output_offset = 0;
                            self.job_output_anchor = ScrollAnchor::Bottom;
                        }
                        KeyCode::Char('c') => {
                            if let Some(id) = self.selected_job_id() {
                                self.dialog = Some(Dialog::ConfirmCancelJob(id));
                            }
                        }
                        KeyCode::Char('C') => {
                            if let Some(id) = self.selected_job_id() {
                                self.dialog = Some(Dialog::SelectCancelSignal {
                                    id,
                                    selected_signal: 0,
                                });
                            }
                        }
                        KeyCode::Char('t') => {
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
                        KeyCode::Char('o') => {
                            self.output_file_view = match self.output_file_view {
                                OutputFileView::Stdout => OutputFileView::Stderr,
                                OutputFileView::Stderr => OutputFileView::Stdout,
                            };
                        }
                        KeyCode::Char('w') => {
                            self.job_output_wrap = !self.job_output_wrap;
                        }
                        _ => {}
                    };
                }
            }
            AppMessage::MouseClick(index) => {
                if self.dialog.is_none() && index < self.jobs.len() {
                    self.job_list_state.select(Some(index));
                }
            }
            AppMessage::MouseWheel {
                target,
                direction,
                amount,
            } => {
                if self.dialog.is_none() {
                    match target {
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
            .set_file_path(self.job_list_state.selected().and_then(|i| {
                self.jobs.get(i).and_then(|j| match self.output_file_view {
                    OutputFileView::Stdout => j.stdout.clone(),
                    OutputFileView::Stderr => j.stderr.clone(),
                })
            }));
    }

    fn selected_job(&self) -> Option<&Job> {
        self.job_list_state
            .selected()
            .and_then(|i| self.jobs.get(i))
    }

    pub(super) fn selected_job_id(&self) -> Option<String> {
        self.selected_job().map(Job::id)
    }

    fn focus_next_panel(&mut self) {
        match self.focus {
            Focus::Jobs => self.focus = Focus::Jobs,
        }
    }

    fn focus_previous_panel(&mut self) {
        match self.focus {
            Focus::Jobs => self.focus = Focus::Jobs,
        }
    }

    fn select_next_job(&mut self) {
        self.job_list_state.select_next();
    }

    fn select_previous_job(&mut self) {
        self.job_list_state.select_previous();
    }

    fn select_first_job(&mut self) {
        self.job_list_state.select_first();
    }

    fn select_last_job(&mut self) {
        self.job_list_state.select_last();
    }

    pub(super) fn scroll_jobs_half_page_down(&mut self) {
        self.job_list_state.scroll_down_by(self.job_list_height / 2);
    }

    pub(super) fn scroll_jobs_half_page_up(&mut self) {
        self.job_list_state.scroll_up_by(self.job_list_height / 2);
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
        if self.jobs.is_empty() {
            return None;
        }
        let rows_area = self.job_list_rows_area();
        if !rect_contains(rows_area, column, row) {
            return None;
        }

        let row_in_list = (row - rows_area.y) as usize;
        let index = self.job_list_state.offset().saturating_add(row_in_list);
        (index < self.jobs.len()).then_some(index)
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

fn signal_index_for_digit(digit: char) -> Option<usize> {
    let value = digit.to_digit(10)? as usize;
    if value == 0 { None } else { Some(value - 1) }
}
