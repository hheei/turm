use super::*;

impl App {
    pub(in crate::app) fn enter_action(&self) -> Option<AppExit> {
        match self.output_panel_mode {
            OutputPanelMode::Stdout => self
                .selected_job()
                .and_then(|job| job.stdout.clone())
                .map(AppExit::OpenEditor),
            OutputPanelMode::Stderr => self
                .selected_job()
                .and_then(|job| job.stderr.clone())
                .map(AppExit::OpenEditor),
            OutputPanelMode::Workdir => self
                .workdir_selected
                .and_then(|index| self.workdir_entries.get(index))
                .map(|entry| {
                    if entry.kind == WorkdirEntryKind::Directory {
                        AppExit::ChangeDirectory(entry.path.clone())
                    } else {
                        AppExit::ChangeDirectory(
                            entry
                                .path
                                .parent()
                                .map(PathBuf::from)
                                .unwrap_or_else(|| entry.path.clone()),
                        )
                    }
                })
                .or_else(|| self.workdir_path.clone().map(AppExit::ChangeDirectory)),
            OutputPanelMode::Collapsed => self
                .selected_job()
                .and_then(Self::derive_workdir_path)
                .map(AppExit::ChangeDirectory),
        }
    }

    pub(in crate::app) fn derive_workdir_path(job: &Job) -> Option<PathBuf> {
        job.workdir
            .clone()
            .or_else(|| {
                job.stdout
                    .as_ref()
                    .and_then(|path| path.parent().map(PathBuf::from))
            })
            .or_else(|| {
                job.stderr
                    .as_ref()
                    .and_then(|path| path.parent().map(PathBuf::from))
            })
            .or_else(|| command_parent_path(&job.command))
    }

    pub(in crate::app) fn reload_workdir_entries(
        &mut self,
        selected_job_changed: bool,
        output_mode_changed: bool,
    ) {
        if self.output_panel_mode != OutputPanelMode::Workdir {
            self.workdir_path = None;
            self.workdir_entries.clear();
            self.workdir_error = None;
            self.workdir_selected = None;
            self.workdir_offset = 0;
            return;
        }

        let derived_path = self.selected_job().and_then(Self::derive_workdir_path);
        let path_changed = derived_path != self.workdir_path;
        let should_reload = output_mode_changed || selected_job_changed || path_changed;

        if !should_reload {
            return;
        }

        self.workdir_path = derived_path.clone();
        self.workdir_entries.clear();
        self.workdir_error = None;
        self.workdir_selected = None;
        self.workdir_offset = 0;

        let Some(path) = derived_path else {
            self.workdir_error = Some("No workdir available".to_string());
            return;
        };

        match load_workdir_entries(&path) {
            Ok(entries) => {
                self.workdir_entries = entries;
                if !self.workdir_entries.is_empty() {
                    self.workdir_selected = Some(0);
                }
            }
            Err(_) => {
                self.workdir_error = Some(format!("Unable to read workdir: {}", path.display()));
            }
        }
    }

    pub(in crate::app) fn workdir_message(&self) -> Option<&str> {
        self.workdir_error.as_deref()
    }

    pub(in crate::app) fn clamp_workdir_selection(&mut self) {
        if self.workdir_entries.is_empty() {
            self.workdir_selected = None;
            return;
        }
        let max_index = self.workdir_entries.len() - 1;
        self.workdir_selected = Some(self.workdir_selected.unwrap_or(0).min(max_index));
    }

    pub(in crate::app) fn clamp_workdir_offset(&mut self) {
        let viewport_height = usize::from(self.output_layout().viewport.height);
        let content_height = if let Some(message) = self.workdir_message() {
            usize::from(!message.is_empty())
        } else {
            self.workdir_entries.len().saturating_add(1)
        };
        let max_offset = content_height.saturating_sub(viewport_height);
        self.workdir_offset = self.workdir_offset.min(max_offset);
        if let Some(selected) = self.workdir_selected {
            if viewport_height == 0 {
                self.workdir_offset = 0;
            } else if selected < self.workdir_offset {
                self.workdir_offset = selected;
            } else {
                let last_visible = self.workdir_offset + viewport_height.saturating_sub(1);
                if selected > last_visible {
                    self.workdir_offset =
                        selected.saturating_add(1).saturating_sub(viewport_height);
                }
            }
        }
    }

    pub(in crate::app) fn move_workdir_selection_up(&mut self, delta: usize) {
        if self.workdir_entries.is_empty() {
            self.workdir_selected = None;
            self.workdir_offset = 0;
            return;
        }

        let next = self.workdir_selected.unwrap_or(0).saturating_sub(delta);
        self.workdir_selected = Some(next);
        self.clamp_workdir_offset();
    }

    pub(in crate::app) fn move_workdir_selection_down(&mut self, delta: usize) {
        if self.workdir_entries.is_empty() {
            self.workdir_selected = None;
            self.workdir_offset = 0;
            return;
        }

        let max_index = self.workdir_entries.len() - 1;
        let next = self
            .workdir_selected
            .unwrap_or(0)
            .saturating_add(delta)
            .min(max_index);
        self.workdir_selected = Some(next);
        self.clamp_workdir_offset();
    }

    pub(in crate::app) fn select_first_workdir_entry(&mut self) {
        if self.workdir_entries.is_empty() {
            self.workdir_selected = None;
            self.workdir_offset = 0;
            return;
        }
        self.workdir_selected = Some(0);
        self.workdir_offset = 0;
    }

    pub(in crate::app) fn select_last_workdir_entry(&mut self) {
        if self.workdir_entries.is_empty() {
            self.workdir_selected = None;
            self.workdir_offset = 0;
            return;
        }
        self.workdir_selected = Some(self.workdir_entries.len() - 1);
        self.clamp_workdir_offset();
    }
}
