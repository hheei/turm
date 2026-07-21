use super::*;

impl App {
    pub(in crate::app) fn move_output_up(&mut self, delta: u16) {
        match self.output_panel_mode {
            OutputPanelMode::Stdout | OutputPanelMode::Stderr => {
                self.scroll_job_output_up_by(delta)
            }
            OutputPanelMode::Workdir => self.move_workdir_selection_up(usize::from(delta)),
            OutputPanelMode::Collapsed => {}
        }
    }

    pub(in crate::app) fn move_output_down(&mut self, delta: u16) {
        match self.output_panel_mode {
            OutputPanelMode::Stdout | OutputPanelMode::Stderr => {
                self.scroll_job_output_down_by(delta)
            }
            OutputPanelMode::Workdir => self.move_workdir_selection_down(usize::from(delta)),
            OutputPanelMode::Collapsed => {}
        }
    }

    pub(in crate::app) fn move_output_to_top(&mut self) {
        match self.output_panel_mode {
            OutputPanelMode::Stdout | OutputPanelMode::Stderr => self.scroll_job_output_to_top(),
            OutputPanelMode::Workdir => self.select_first_workdir_entry(),
            OutputPanelMode::Collapsed => {}
        }
    }

    pub(in crate::app) fn move_output_to_bottom(&mut self) {
        match self.output_panel_mode {
            OutputPanelMode::Stdout | OutputPanelMode::Stderr => self.scroll_job_output_to_bottom(),
            OutputPanelMode::Workdir => self.select_last_workdir_entry(),
            OutputPanelMode::Collapsed => {}
        }
    }

    pub(in crate::app) fn move_output_half_page_up(&mut self) {
        self.move_output_up(self.output_page_step());
    }

    pub(in crate::app) fn output_page_step(&self) -> u16 {
        self.output_layout()
            .viewport
            .height
            .saturating_div(2)
            .max(1)
    }

    pub(in crate::app) fn scroll_output_left(&mut self, delta: u16) {
        if !self.is_output_horizontally_scrollable() {
            self.output_scroll_x = 0;
            return;
        }
        self.output_scroll_x = self.output_scroll_x.saturating_sub(delta);
    }

    pub(in crate::app) fn scroll_output_right(&mut self, delta: u16) {
        if !self.is_output_horizontally_scrollable() {
            self.output_scroll_x = 0;
            return;
        }
        self.output_scroll_x = self
            .output_scroll_x
            .saturating_add(delta)
            .min(self.max_output_scroll_x());
    }

    pub(in crate::app) fn scroll_output_page_left(&mut self) {
        self.scroll_output_left(self.output_horizontal_page_step());
    }

    pub(in crate::app) fn scroll_output_page_right(&mut self) {
        self.scroll_output_right(self.output_horizontal_page_step());
    }

    pub(in crate::app) fn output_horizontal_page_step(&self) -> u16 {
        self.output_layout().viewport.width.saturating_div(2).max(1)
    }

    pub(in crate::app) fn output_inner_area(&self) -> Rect {
        Rect::new(
            self.job_output_area.x.saturating_add(2),
            self.job_output_area.y.saturating_add(1),
            self.job_output_area.width.saturating_sub(3),
            self.job_output_area.height.saturating_sub(2),
        )
    }

    pub(in crate::app) fn output_layout(&self) -> OutputLayout {
        let inner = self.output_inner_area();
        let content_width = self.output_content_width();
        let mut show_vertical = false;
        let mut show_horizontal = false;

        loop {
            let viewport = Rect::new(
                inner.x,
                inner.y,
                inner.width.saturating_sub(u16::from(show_vertical)),
                inner.height.saturating_sub(u16::from(show_horizontal)),
            );
            let content_height = self.output_content_height_for(viewport.width);
            let next_vertical =
                viewport.height > 0 && content_height > usize::from(viewport.height);
            let next_horizontal = !self.job_output_wrap
                && viewport.width > 0
                && content_width > usize::from(viewport.width);
            if next_vertical == show_vertical && next_horizontal == show_horizontal {
                return OutputLayout {
                    viewport,
                    show_vertical,
                    show_horizontal,
                };
            }
            show_vertical = next_vertical;
            show_horizontal = next_horizontal;
        }
    }

    pub(in crate::app) fn is_output_horizontally_scrollable(&self) -> bool {
        !self.job_output_wrap && !self.output_panel_mode.is_collapsed()
    }

    pub(in crate::app) fn output_content_height_for(&self, viewport_width: u16) -> usize {
        if viewport_width == 0 {
            return 0;
        }
        match self.output_panel_mode {
            OutputPanelMode::Stdout | OutputPanelMode::Stderr => match self.job_output.as_deref() {
                Ok(content) => job_output_line_count(
                    content,
                    usize::from(viewport_width),
                    self.job_output_wrap,
                ),
                Err(error) => {
                    job_output_line_count(&error.to_string(), usize::from(viewport_width), true)
                }
            },
            OutputPanelMode::Workdir => {
                if let Some(message) = self.workdir_message() {
                    usize::from(!message.is_empty())
                } else {
                    self.workdir_entries.len().saturating_add(1)
                }
            }
            OutputPanelMode::Collapsed => 0,
        }
    }

    pub(in crate::app) fn output_content_width(&self) -> usize {
        match self.output_panel_mode {
            OutputPanelMode::Stdout | OutputPanelMode::Stderr => self
                .job_output
                .as_deref()
                .map(max_line_chars)
                .unwrap_or_else(|error| max_line_chars(&error.to_string())),
            OutputPanelMode::Workdir => {
                if let Some(message) = self.workdir_message() {
                    message.chars().count()
                } else {
                    self.workdir_entries
                        .iter()
                        .map(|entry| workdir_entry_label(entry).chars().count())
                        .max()
                        .unwrap_or(0)
                }
            }
            OutputPanelMode::Collapsed => 0,
        }
    }

    pub(in crate::app) fn max_output_scroll_x(&self) -> u16 {
        if !self.is_output_horizontally_scrollable() {
            return 0;
        }
        let viewport_width = usize::from(self.output_layout().viewport.width);
        if viewport_width == 0 {
            return 0;
        }
        self.output_content_width()
            .saturating_sub(viewport_width)
            .min(u16::MAX as usize) as u16
    }

    pub(in crate::app) fn sync_output_state(&mut self) {
        if !self.is_output_horizontally_scrollable() {
            self.output_scroll_x = 0;
        } else {
            self.output_scroll_x = self.output_scroll_x.min(self.max_output_scroll_x());
        }

        match self.output_panel_mode {
            OutputPanelMode::Stdout | OutputPanelMode::Stderr => self.clamp_job_output_offset(),
            OutputPanelMode::Workdir => {
                self.clamp_workdir_selection();
                self.clamp_workdir_offset();
            }
            OutputPanelMode::Collapsed => {}
        }
    }

    pub(in crate::app) fn scroll_job_output_to_top(&mut self) {
        self.job_output_offset = 0;
        self.job_output_anchor = ScrollAnchor::Top;
    }

    pub(in crate::app) fn scroll_job_output_to_bottom(&mut self) {
        self.job_output_offset = 0;
        self.job_output_anchor = ScrollAnchor::Bottom;
    }

    pub(in crate::app) fn scroll_job_output_down_by(&mut self, delta: u16) {
        match self.job_output_anchor {
            ScrollAnchor::Top => {
                self.job_output_offset = self.job_output_offset.saturating_add(delta)
            }
            ScrollAnchor::Bottom => {
                self.job_output_offset = self.job_output_offset.saturating_sub(delta)
            }
        }
        self.clamp_job_output_offset();
    }

    pub(in crate::app) fn scroll_job_output_up_by(&mut self, delta: u16) {
        match self.job_output_anchor {
            ScrollAnchor::Top => {
                self.job_output_offset = self.job_output_offset.saturating_sub(delta)
            }
            ScrollAnchor::Bottom => {
                self.job_output_offset = self.job_output_offset.saturating_add(delta)
            }
        }
        self.clamp_job_output_offset();
    }

    pub(in crate::app) fn clamp_job_output_offset(&mut self) {
        self.job_output_offset = self.job_output_offset.min(self.max_job_output_offset());
    }

    pub(in crate::app) fn max_job_output_offset(&self) -> u16 {
        if !matches!(
            self.output_panel_mode,
            OutputPanelMode::Stdout | OutputPanelMode::Stderr
        ) {
            return 0;
        }

        let layout = self.output_layout();
        let viewport_lines = usize::from(layout.viewport.height);
        let viewport_width = layout.viewport.width;
        if viewport_lines == 0 || viewport_width == 0 {
            return 0;
        }

        let content_lines = match self.job_output.as_deref() {
            Ok(content) => {
                job_output_line_count(content, usize::from(viewport_width), self.job_output_wrap)
            }
            Err(error) => {
                job_output_line_count(&error.to_string(), usize::from(viewport_width), true)
            }
        };

        content_lines
            .saturating_sub(viewport_lines)
            .min(u16::MAX as usize) as u16
    }
}
