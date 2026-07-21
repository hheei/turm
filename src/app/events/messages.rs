use super::*;

impl App {
    pub(in crate::app) fn handle(&mut self, msg: AppMessage) {
        let previous_selected_job_id = self.selected_job_id();
        let previous_output_panel_mode = self.output_panel_mode;

        match msg {
            AppMessage::Jobs(jobs) => {
                let selected_id = self.selected_job_id();
                let fallback_index = self.job_list_state.selected();

                self.jobs = jobs;
                self.sort_jobs();
                self.restore_selection_by_job_id(selected_id, fallback_index);
            }
            AppMessage::JobOutput(content) => self.job_output = content,
            AppMessage::ResourcesUpdated(resources) => {
                let old_offset = self.resource_table_state.offset();
                self.resources = resources;
                if old_offset > 0 && old_offset >= self.resources.len().saturating_sub(1) {
                    self.resource_table_state = TableState::new();
                }
            }
            AppMessage::ResourceWatcherError(_) => {}
            AppMessage::Key(key) => {
                if self.dialog.is_some() {
                    let mut close_dialog = false;
                    let mut scancel_request = None;
                    let mut timelimit_request = None;
                    let mut rename_request = None;
                    let mut filter_to_apply = None;
                    let mut clipboard_copy = None;
                    let mut command_failure = None;

                    match self.dialog.as_mut().expect("dialog must exist") {
                        Dialog::ConfirmCancelJob {
                            id,
                            signal,
                            selected,
                            ..
                        } => match cancel_confirmation_action(key, *selected) {
                            CancelConfirmationAction::Confirm => {
                                scancel_request = Some((id.clone(), signal.as_deref()));
                                close_dialog = true;
                            }
                            CancelConfirmationAction::Cancel => {
                                close_dialog = true;
                            }
                            CancelConfirmationAction::Select(choice) => {
                                *selected = choice;
                            }
                            CancelConfirmationAction::Ignore => {}
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
                        Dialog::EditJobName { id, input } => match key.code {
                            KeyCode::Enter => {
                                let job_name = input.value().trim();
                                if !job_name.is_empty() {
                                    rename_request = Some((id.clone(), job_name.to_string()));
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
                    if let Some((id, job_name)) = rename_request {
                        command_failure = execute_scontrol_update_job_name(&id, &job_name).err();
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
                        KeyCode::Enter => {
                            self.pending_exit = self.enter_action();
                        }
                        KeyCode::Tab => self.cycle_output_panel_mode(),
                        KeyCode::Left => self.focus_previous_panel(),
                        KeyCode::Right => self.focus_next_panel(),
                        KeyCode::Char('k') | KeyCode::Up => match self.focus {
                            Focus::Resources => self.select_previous_resource(),
                            Focus::Jobs => self.select_previous_job(),
                            Focus::Details => {}
                            Focus::Log => self.move_output_up(1),
                        },
                        KeyCode::Down => match self.focus {
                            Focus::Resources => self.select_next_resource(),
                            Focus::Jobs => self.select_next_job(),
                            Focus::Details => {}
                            Focus::Log => self.move_output_down(1),
                        },
                        KeyCode::Char('j') => match self.focus {
                            Focus::Resources => self.select_next_resource(),
                            Focus::Jobs => self.update_job_sort(JobSortField::Id),
                            Focus::Details => {}
                            Focus::Log => self.move_output_down(1),
                        },
                        KeyCode::Char('g') => match self.focus {
                            Focus::Resources => self.select_first_resource(),
                            Focus::Jobs => self.select_first_job(),
                            Focus::Details => {}
                            Focus::Log => self.move_output_to_top(),
                        },
                        KeyCode::Char('G') => match self.focus {
                            Focus::Resources => self.select_last_resource(),
                            Focus::Jobs => self.select_last_job(),
                            Focus::Details => {}
                            Focus::Log => self.move_output_to_bottom(),
                        },
                        KeyCode::Char('f') => {
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
                                    Focus::Log => self.move_output_half_page_up(),
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
                            } else {
                                self.details_visible = !self.details_visible;
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
                            self.move_output_down(delta);
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
                            self.move_output_up(delta);
                        }
                        KeyCode::Home if matches!(self.focus, Focus::Log) => {
                            self.move_output_to_top();
                        }
                        KeyCode::End if matches!(self.focus, Focus::Log) => {
                            self.move_output_to_bottom();
                        }
                        KeyCode::Char('H') if matches!(self.focus, Focus::Log) => {
                            self.scroll_output_page_left();
                        }
                        KeyCode::Char('L') if matches!(self.focus, Focus::Log) => {
                            self.scroll_output_page_right();
                        }
                        KeyCode::Char('c')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            if let Some(value) = self.selected_mouse_text() {
                                self.pending_clipboard_copy = Some(value);
                                self.clear_mouse_selection();
                            }
                        }
                        KeyCode::Char('c') if matches!(self.focus, Focus::Jobs) => {
                            self.dialog = self.copy_job_output_directory_dialog();
                        }
                        KeyCode::Char('r')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL)
                                || !matches!(self.focus, Focus::Resources) =>
                        {
                            if let Some(job) = self.selected_job() {
                                self.dialog = Some(Dialog::EditJobName {
                                    id: job.id(),
                                    input: Input::new(job.name.clone()),
                                });
                            }
                        }
                        KeyCode::Char('t')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            if let Some(job) = self.selected_job() {
                                self.dialog = Some(Dialog::EditTimeLimit {
                                    id: job.id(),
                                    input: Input::new(job.time_limit.clone()),
                                });
                            }
                        }
                        KeyCode::Char('t') if matches!(self.focus, Focus::Jobs) => {
                            self.update_job_sort(JobSortField::Time);
                        }
                        KeyCode::Char('w') if matches!(self.focus, Focus::Log) => {
                            self.job_output_wrap = !self.job_output_wrap;
                            if self.job_output_wrap {
                                self.output_scroll_x = 0;
                            }
                        }
                        _ => {}
                    };
                }
            }
            AppMessage::MouseClick { column, row } => {
                if self.dialog.is_none() {
                    if rect_contains(self.job_details_area, column, row) {
                        self.focus = Focus::Jobs;
                    } else if rect_contains(self.resource_area, column, row) {
                        self.focus = Focus::Resources;
                        self.select_resource_at(column, row);
                    } else if rect_contains(self.job_list_area, column, row) {
                        self.focus = Focus::Jobs;
                        self.select_job_at(column, row);
                    } else if rect_contains(self.job_output_area, column, row) {
                        self.focus = Focus::Log;
                        self.select_workdir_at(column, row);
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
                            MouseWheelDirection::Up => self.move_output_up(amount),
                            MouseWheelDirection::Down => self.move_output_down(amount),
                        },
                    }
                }
            }
        }

        if self.output_panel_mode.is_collapsed() && matches!(self.focus, Focus::Log) {
            self.focus = Focus::Jobs;
        }

        let selected_job_changed = self.selected_job_id() != previous_selected_job_id;
        let output_mode_changed = previous_output_panel_mode != self.output_panel_mode;
        let switched_to_file_output = output_mode_changed
            && matches!(
                self.output_panel_mode,
                OutputPanelMode::Stdout | OutputPanelMode::Stderr
            );
        if output_mode_changed {
            self.output_scroll_x = 0;
        }
        if switched_to_file_output
            || (selected_job_changed
                && matches!(
                    self.output_panel_mode,
                    OutputPanelMode::Stdout | OutputPanelMode::Stderr
                ))
        {
            self.scroll_job_output_to_bottom();
        }

        self.reload_workdir_entries(selected_job_changed, output_mode_changed);
        self.sync_output_state();
        self.job_output_watcher.set_file_path(
            self.selected_job()
                .and_then(|job| watched_output_path(job, self.output_panel_mode)),
        );
    }
}
