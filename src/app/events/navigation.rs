use super::*;

impl App {
    pub(in crate::app) fn cycle_output_panel_mode(&mut self) {
        if !self.output_can_expand {
            return;
        }
        self.output_panel_mode = self.output_panel_mode.next();

        if self.output_panel_mode.is_collapsed() && matches!(self.focus, Focus::Log) {
            self.focus = Focus::Jobs;
        }
    }

    pub(in crate::app) fn focus_previous_panel(&mut self) {
        self.focus = match self.focus {
            Focus::Resources => {
                if self.output_can_expand && !self.output_panel_mode.is_collapsed() {
                    Focus::Log
                } else {
                    Focus::Jobs
                }
            }
            Focus::Jobs | Focus::Details => Focus::Resources,
            Focus::Log => Focus::Jobs,
        };
        self.ensure_focused_selection();
    }

    pub(in crate::app) fn focus_next_panel(&mut self) {
        self.focus = match self.focus {
            Focus::Resources => Focus::Jobs,
            Focus::Jobs | Focus::Details => {
                if self.output_can_expand && !self.output_panel_mode.is_collapsed() {
                    Focus::Log
                } else {
                    Focus::Resources
                }
            }
            Focus::Log => Focus::Resources,
        };
        self.ensure_focused_selection();
    }

    fn ensure_focused_selection(&mut self) {
        match self.focus {
            Focus::Resources
                if self.resource_table_state.selected().is_none() && !self.resources.is_empty() =>
            {
                self.resource_table_state.select(Some(0));
            }
            Focus::Jobs
                if self.job_list_state.selected().is_none()
                    && !self.visible_job_indices().is_empty() =>
            {
                self.job_list_state.select(Some(0));
            }
            _ => {}
        }
    }

    pub(in crate::app) fn select_next_job(&mut self) {
        if !self.visible_job_indices().is_empty() {
            self.job_list_state.select_next();
        }
    }

    pub(in crate::app) fn select_previous_job(&mut self) {
        if !self.visible_job_indices().is_empty() {
            self.job_list_state.select_previous();
        }
    }

    pub(in crate::app) fn select_first_job(&mut self) {
        if !self.visible_job_indices().is_empty() {
            self.job_list_state.select_first();
        }
    }

    pub(in crate::app) fn select_last_job(&mut self) {
        if !self.visible_job_indices().is_empty() {
            self.job_list_state.select_last();
        }
    }

    pub(in crate::app) fn select_next_resource(&mut self) {
        if !self.resources.is_empty() {
            self.resource_table_state.select_next();
        }
    }

    pub(in crate::app) fn select_previous_resource(&mut self) {
        if !self.resources.is_empty() {
            self.resource_table_state.select_previous();
        }
    }

    pub(in crate::app) fn select_first_resource(&mut self) {
        if !self.resources.is_empty() {
            self.resource_table_state.select_first();
        }
    }

    pub(in crate::app) fn select_last_resource(&mut self) {
        if !self.resources.is_empty() {
            self.resource_table_state.select_last();
        }
    }

    pub(in crate::app) fn select_job_at(&mut self, column: u16, row: u16) {
        let rows = Rect::new(
            self.job_list_area.x.saturating_add(2),
            self.job_list_area.y.saturating_add(2),
            self.job_list_area.width.saturating_sub(4),
            self.job_list_height,
        );
        if !rect_contains(rows, column, row) {
            return;
        }
        let index = self
            .job_list_state
            .offset()
            .saturating_add(usize::from(row.saturating_sub(rows.y)));
        if index < self.visible_job_indices().len() {
            self.job_list_state.select(Some(index));
        }
    }

    pub(in crate::app) fn select_resource_at(&mut self, column: u16, row: u16) {
        let rows = Rect::new(
            self.resource_area.x.saturating_add(2),
            self.resource_area.y.saturating_add(2),
            self.resource_area.width.saturating_sub(4),
            self.resource_list_height,
        );
        if !rect_contains(rows, column, row) || self.resources.is_empty() {
            return;
        }
        let index = self
            .resource_table_state
            .offset()
            .saturating_add(usize::from(row.saturating_sub(rows.y)));
        if index < self.resources.len() {
            self.resource_table_state.select(Some(index));
        }
    }

    pub(in crate::app) fn select_workdir_at(&mut self, column: u16, row: u16) {
        if self.output_panel_mode != OutputPanelMode::Workdir {
            return;
        }
        let viewport = self.output_layout().viewport;
        if !rect_contains(viewport, column, row) {
            return;
        }
        let index = self
            .workdir_offset
            .saturating_add(usize::from(row.saturating_sub(viewport.y)))
            .saturating_sub(1);
        if index < self.workdir_entries.len() {
            self.workdir_selected = Some(index);
        }
    }

    pub(in crate::app) fn scroll_jobs_half_page_up(&mut self) {
        if !self.visible_job_indices().is_empty() {
            self.job_list_state.scroll_up_by(self.job_list_height / 2);
        }
    }

    pub(in crate::app) fn job_list_rows_area(&self) -> Rect {
        Rect::new(
            self.job_list_area.x.saturating_add(1),
            self.job_list_area.y.saturating_add(2),
            self.job_list_area.width.saturating_sub(2),
            self.job_list_height,
        )
    }
}
