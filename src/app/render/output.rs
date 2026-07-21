use super::*;

impl App {
    pub(super) fn render_output(&mut self, f: &mut Frame, area: Option<Rect>) {
        let Some(log_area) = area else {
            self.job_output_area = Rect::default();
            return;
        };
        self.job_output_area = log_area;
        let log_block = Block::default()
            .title(Line::from(vec![
                Span::styled("─", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!(" {} ", self.output_panel_mode.title()),
                    Style::default().fg(if self.focus == Focus::Log {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }),
                ),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .padding(Padding::left(1))
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = log_block.inner(log_area);
        let layout = self.output_layout();
        let viewport = layout.viewport;

        let log = match self.output_panel_mode {
            OutputPanelMode::Stdout | OutputPanelMode::Stderr => {
                let text = match self.job_output.as_deref() {
                    Ok(content) => fit_text(
                        content,
                        usize::from(viewport.height),
                        usize::from(viewport.width),
                        self.job_output_anchor,
                        usize::from(self.job_output_offset),
                        self.job_output_wrap,
                        usize::from(self.output_scroll_x),
                    ),
                    Err(error) => Text::from(error.to_string()),
                };
                Paragraph::new(text).style(match self.job_output {
                    Ok(_) => Style::default(),
                    Err(_) => Style::default().fg(Color::Red),
                })
            }
            OutputPanelMode::Workdir => {
                let text = render_workdir_text(
                    &self.workdir_entries,
                    self.workdir_message(),
                    self.workdir_offset,
                    usize::from(viewport.height),
                    usize::from(viewport.width),
                    usize::from(self.output_scroll_x),
                    self.workdir_selected.filter(|_| self.focus == Focus::Log),
                );
                Paragraph::new(text)
            }
            OutputPanelMode::Collapsed => unreachable!(),
        };
        f.render_widget(log_block, log_area);
        f.render_widget(log, viewport);

        if layout.show_vertical {
            let content_height = self.output_content_height_for(viewport.width);
            let max_offset = content_height.saturating_sub(usize::from(viewport.height));
            let position = match self.output_panel_mode {
                OutputPanelMode::Stdout | OutputPanelMode::Stderr => match self.job_output_anchor {
                    ScrollAnchor::Top => usize::from(self.job_output_offset),
                    ScrollAnchor::Bottom => {
                        max_offset.saturating_sub(usize::from(self.job_output_offset))
                    }
                },
                OutputPanelMode::Workdir => self.workdir_offset,
                OutputPanelMode::Collapsed => unreachable!(),
            };
            let mut scrollbar_state = ScrollbarState::new(max_offset.saturating_add(1))
                .position(position)
                .viewport_content_length(usize::from(viewport.height));
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_symbol(VERTICAL_SCROLLBAR_THUMB)
                .track_symbol(Some(VERTICAL_SCROLLBAR_TRACK))
                .begin_symbol(None)
                .end_symbol(None);
            let area = Rect::new(
                inner.x,
                inner.y,
                inner.width,
                inner
                    .height
                    .saturating_sub(u16::from(layout.show_horizontal)),
            );
            f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }

        if layout.show_horizontal {
            let max_offset = self
                .output_content_width()
                .saturating_sub(usize::from(viewport.width));
            let mut scrollbar_state = ScrollbarState::new(max_offset.saturating_add(1))
                .position(usize::from(self.output_scroll_x))
                .viewport_content_length(usize::from(viewport.width));
            let scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                .thumb_symbol(OUTPUT_HORIZONTAL_SCROLLBAR_THUMB)
                .track_symbol(Some(OUTPUT_HORIZONTAL_SCROLLBAR_TRACK))
                .begin_symbol(None)
                .end_symbol(None);
            let area = Rect::new(
                inner.x.saturating_add(u16::from(layout.show_vertical)),
                inner.y,
                inner.width.saturating_sub(u16::from(layout.show_vertical)),
                inner.height,
            );
            f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }
}
