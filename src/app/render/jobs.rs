use super::*;

impl App {
    pub(super) fn render_jobs(&mut self, f: &mut Frame, area: Rect) {
        self.job_list_height = area.height.saturating_sub(3);
        self.job_list_area = area;
        let viewport_height = usize::from(self.job_list_height);
        let (content_height, scroll_offset, table) = {
            let visible = self.visible_job_indices();
            let content_height = visible.len();
            let max_offset = content_height.saturating_sub(viewport_height);
            let mut scroll_offset = self.job_list_state.offset().min(max_offset);
            if let Some(selected) = self.job_list_state.selected() {
                if selected < scroll_offset {
                    scroll_offset = selected;
                } else if viewport_height > 0 && selected >= scroll_offset + viewport_height {
                    scroll_offset = selected + 1 - viewport_height;
                }
                scroll_offset = scroll_offset.min(max_offset);
            }

            let max_id_len = visible
                .iter()
                .map(|&i| self.jobs[i].id().len())
                .max()
                .unwrap_or(0);
            let max_user_len = visible
                .iter()
                .map(|&i| self.jobs[i].user.len())
                .max()
                .unwrap_or(0);
            let max_name_len = visible
                .iter()
                .map(|&i| self.jobs[i].name.chars().count())
                .max()
                .unwrap_or(0);
            let max_partition_len = visible
                .iter()
                .map(|&i| self.jobs[i].partition.len())
                .max()
                .unwrap_or(0);
            let max_start_time_len = visible
                .iter()
                .map(|&i| self.jobs[i].start_time.len())
                .max()
                .unwrap_or(0);
            let max_duration_len = visible
                .iter()
                .map(|&i| self.jobs[i].time.len())
                .max()
                .unwrap_or(0);
            let max_state_compact_len = visible
                .iter()
                .map(|&i| self.jobs[i].state_compact.len())
                .max()
                .unwrap_or(0);
            let rows = visible
                .iter()
                .skip(scroll_offset)
                .take(viewport_height)
                .map(|&index| {
                    let j = &self.jobs[index];
                    Row::new(vec![
                        Cell::from(j.state_compact.as_str()),
                        Cell::from(Line::from(Span::styled(
                            j.partition.as_str(),
                            Style::default().fg(Color::Blue),
                        ))),
                        Cell::from(Line::from(Span::styled(
                            j.id(),
                            Style::default().fg(Color::Yellow),
                        ))),
                        Cell::from(j.name.as_str()),
                        Cell::from(Line::from(Span::styled(
                            j.user.as_str(),
                            Style::default().fg(Color::Green),
                        ))),
                        Cell::from(Line::from(Span::styled(
                            j.start_time.as_str(),
                            Style::default().fg(Color::Red),
                        ))),
                        Cell::from(j.time.as_str()),
                    ])
                    .style(if j.is_terminal() {
                        Style::default().add_modifier(Modifier::DIM)
                    } else {
                        Style::default()
                    })
                })
                .collect::<Vec<_>>();
            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled("─", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        jobs_title(
                            area.width,
                            visible.len(),
                            self.jobs.len(),
                            &self.active_filter,
                        ),
                        Style::default().fg(if self.focus == Focus::Jobs {
                            Color::Green
                        } else {
                            Color::DarkGray
                        }),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(Color::DarkGray));
            let duration_indicator = match self.sort_indicator(JobSortField::Duration) {
                "" => " ",
                value => value,
            };
            let header = Row::new(vec![
                sort_header_cell("s", "t", self.sort_indicator(JobSortField::State)),
                sort_header_cell(
                    "p",
                    "artition",
                    self.sort_indicator(JobSortField::Partition),
                ),
                sort_header_cell("j", "obid", self.sort_indicator(JobSortField::Id)),
                sort_header_cell("n", "ame", self.sort_indicator(JobSortField::Name)),
                Cell::from(Line::from(Span::styled(
                    "user",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))),
                sort_header_cell("t", "ime", self.sort_indicator(JobSortField::Time)),
                Cell::from(Line::from(vec![
                    Span::styled(
                        "d",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "u",
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "ration",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        duration_indicator,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])),
            ]);
            let widths = [
                Constraint::Length((max_state_compact_len.max(3) as u16).min(8)),
                Constraint::Length((max_partition_len.max(10) as u16).min(12)),
                Constraint::Length((max_id_len.max(6) as u16).min(12)),
                Constraint::Length((max_name_len.max(5) as u16).min(25)),
                Constraint::Length((max_user_len.max(5) as u16).min(12)),
                Constraint::Length((max_start_time_len.max(5) as u16).min(19)),
                Constraint::Length((max_duration_len.max(9) as u16).min(12)),
            ];
            let table = Table::new(rows, widths)
                .header(header)
                .block(block)
                .column_spacing(1)
                .row_highlight_style(if self.focus == Focus::Jobs {
                    Style::default().bg(Color::Green).fg(Color::Black)
                } else {
                    Style::default()
                });
            (content_height, scroll_offset, table)
        };
        let selected = self.job_list_state.selected();
        let local_selected = selected
            .filter(|&index| {
                index >= scroll_offset && index < scroll_offset.saturating_add(viewport_height)
            })
            .map(|index| index - scroll_offset);
        let mut local_state = TableState::default().with_selected(local_selected);
        f.render_stateful_widget(table, area, &mut local_state);
        self.job_list_state = TableState::default()
            .with_selected(selected)
            .with_offset(scroll_offset);
        let job_list_rows_area = self.job_list_rows_area();
        if viewport_height > 0 && content_height > viewport_height {
            let max_offset = content_height.saturating_sub(viewport_height);
            let mut scrollbar_state = ScrollbarState::new(max_offset.saturating_add(1))
                .position(scroll_offset)
                .viewport_content_length(viewport_height);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalLeft)
                .thumb_symbol(VERTICAL_SCROLLBAR_THUMB)
                .track_symbol(Some(VERTICAL_SCROLLBAR_TRACK))
                .begin_symbol(None)
                .end_symbol(None);
            f.render_stateful_widget(scrollbar, job_list_rows_area, &mut scrollbar_state);
        }
    }
}
