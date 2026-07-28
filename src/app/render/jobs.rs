use super::*;

impl App {
    pub(super) fn render_jobs(&mut self, f: &mut Frame, area: Rect) {
        let visible_job_indices = self.visible_job_indices();
        let visible_jobs = visible_job_indices
            .iter()
            .map(|&index| &self.jobs[index])
            .collect::<Vec<_>>();

        let max_id_len = visible_jobs.iter().map(|j| j.id().len()).max().unwrap_or(0);
        let max_user_len = visible_jobs.iter().map(|j| j.user.len()).max().unwrap_or(0);
        let max_name_len = visible_jobs
            .iter()
            .map(|j| j.name.chars().count())
            .max()
            .unwrap_or(0);
        let max_partition_len = visible_jobs
            .iter()
            .map(|j| j.partition.len())
            .max()
            .unwrap_or(0);
        let max_start_time_len = visible_jobs
            .iter()
            .map(|j| j.start_time.len())
            .max()
            .unwrap_or(0);
        let max_duration_len = visible_jobs.iter().map(|j| j.time.len()).max().unwrap_or(0);
        let max_state_compact_len = visible_jobs
            .iter()
            .map(|j| j.state_compact.len())
            .max()
            .unwrap_or(0);

        let jobs_block = Block::default()
            .title(Line::from(vec![
                Span::styled("─", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    jobs_title(
                        area.width,
                        visible_job_indices.len(),
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
        let duration_indicator = self.sort_indicator(JobSortField::Duration);
        let duration_indicator = if duration_indicator.is_empty() {
            " "
        } else {
            duration_indicator
        };
        let job_header = Row::new(vec![
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
        let job_table_widths = [
            Constraint::Length((max_state_compact_len.max(3) as u16).min(8)),
            Constraint::Length((max_partition_len.max(10) as u16).min(12)),
            Constraint::Length((max_id_len.max(6) as u16).min(12)),
            Constraint::Length((max_name_len.max(5) as u16).min(25)),
            Constraint::Length((max_user_len.max(5) as u16).min(12)),
            Constraint::Length((max_start_time_len.max(5) as u16).min(19)),
            Constraint::Length((max_duration_len.max(9) as u16).min(12)),
        ];
        let jobs: Vec<Row> = visible_jobs
            .iter()
            .map(|j| {
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
            .collect();
        let job_table = Table::new(jobs, job_table_widths)
            .header(job_header)
            .block(jobs_block)
            .column_spacing(1)
            .row_highlight_style(if self.focus == Focus::Jobs {
                Style::default().bg(Color::Green).fg(Color::Black)
            } else {
                Style::default()
            });
        f.render_stateful_widget(job_table, area, &mut self.job_list_state);
        self.job_list_height = area.height.saturating_sub(3);
        self.job_list_area = area;

        let job_list_viewport_height = usize::from(self.job_list_height);
        let job_list_content_height = visible_job_indices.len();
        let job_list_scroll_offset = self.job_list_state.offset();
        let job_list_rows_area = self.job_list_rows_area();
        if job_list_viewport_height > 0 && job_list_content_height > job_list_viewport_height {
            let max_offset = job_list_content_height.saturating_sub(job_list_viewport_height);
            let mut job_list_scrollbar_state = ScrollbarState::new(max_offset.saturating_add(1))
                .position(job_list_scroll_offset)
                .viewport_content_length(job_list_viewport_height);
            let job_list_scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalLeft)
                .thumb_symbol(VERTICAL_SCROLLBAR_THUMB)
                .track_symbol(Some(VERTICAL_SCROLLBAR_TRACK))
                .begin_symbol(None)
                .end_symbol(None);
            f.render_stateful_widget(
                job_list_scrollbar,
                job_list_rows_area,
                &mut job_list_scrollbar_state,
            );
        }
    }
}
