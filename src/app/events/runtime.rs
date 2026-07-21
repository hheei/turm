use super::*;

impl App {
    pub fn run<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> io::Result<Option<AppExit>> {
        terminal.draw(|f| self.ui(f))?;

        loop {
            let notice_timeout = self
                .clipboard_notice_until
                .map(|until| after(until.saturating_duration_since(Instant::now())))
                .unwrap_or_else(never);
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
                    recv(notice_timeout) -> _ => {
                        self.clipboard_notice_until = None;
                        (false, true)
                    }
                }
            };
            if should_quit {
                return Ok(self.pending_exit.take());
            }

            if should_draw {
                terminal.draw(|f| self.ui(f))?;
            }
            self.flush_pending_clipboard_copy()?;
        }
    }

    pub(in crate::app) fn try_recv_input_event(&mut self) -> Option<Event> {
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

    pub(in crate::app) fn handle_input_event(&mut self, event: Event) -> (bool, bool) {
        match event {
            Event::Key(key) => {
                if self.dialog.is_none() && key.code == KeyCode::Char('q') {
                    return (true, false);
                }
                self.handle(AppMessage::Key(key));
                (self.pending_exit.is_some(), self.pending_exit.is_none())
            }
            Event::Paste(_) => (false, false),
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if self.dialog.is_some() {
                        return (false, false);
                    }
                    self.begin_mouse_selection(mouse.column, mouse.row);
                    self.handle(AppMessage::MouseClick {
                        column: mouse.column,
                        row: mouse.row,
                    });
                    (false, true)
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    if self.dialog.is_some() {
                        return (false, false);
                    }
                    self.update_mouse_selection(mouse.column, mouse.row);
                    (false, true)
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    if self.dialog.is_none() {
                        self.update_mouse_selection(mouse.column, mouse.row);
                        let dragged = self
                            .mouse_selection
                            .is_some_and(|selection| selection.dragged);
                        if dragged {
                            self.pending_clipboard_copy = self.selected_mouse_text();
                            self.clipboard_notice_until =
                                Some(Instant::now() + Duration::from_secs(2));
                        }
                        self.clear_mouse_selection();
                    }
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

    pub(in crate::app) fn mouse_scroll_target(
        &self,
        column: u16,
        row: u16,
    ) -> Option<MouseScrollTarget> {
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
}
