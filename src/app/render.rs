use super::*;

impl App {
    pub(super) fn ui(&mut self, f: &mut Frame) {
        let top_row_height: u16 = 8;

        let content_help = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)].as_ref())
            .split(f.area());

        let top_bottom = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(top_row_height), Constraint::Min(3)].as_ref())
            .split(content_help[0]);

        let top_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(top_bottom[0]);

        let bottom_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(top_bottom[1]);
        let help_options = match &self.dialog {
            Some(Dialog::FilterJobs { .. }) => vec![
                ("mode", "filter"),
                ("enter", "apply"),
                ("esc", "cancel"),
                ("ctrl+u", "clear"),
            ],
            Some(Dialog::CopyJobOutputDirectory { .. }) => vec![
                ("mode", "copy"),
                ("c", "dir-url"),
                ("d", "dir-name"),
                ("esc", "cancel"),
            ],
            _ => match self.focus {
                Focus::Resources => vec![
                    ("mode", "resources"),
                    ("[/]", "focus"),
                    ("j/k", "move"),
                    ("g/G", "top/bottom"),
                    ("q", "quit"),
                ],
                Focus::Jobs => vec![
                    ("[/]", "focus"),
                    ("f", "filter"),
                    ("c", "copy"),
                    ("^d", "cancel"),
                    ("q", "quit"),
                ],
                Focus::Details => vec![
                    ("mode", "details"),
                    ("[/]", "focus"),
                    ("^d", "cancel"),
                    ("q", "quit"),
                ],
                Focus::Log => vec![
                    ("mode", "log"),
                    ("[/]", "focus"),
                    ("^d", "cancel"),
                    ("j/k", "scroll"),
                    ("g/G", "top/bottom"),
                    ("o", "output"),
                    ("w", "wrap"),
                    ("q", "quit"),
                ],
            },
        };
        let blue_style = Style::default().fg(Color::Blue);
        let light_blue_style = Style::default().fg(Color::LightBlue);
        let white_style = Style::default().fg(Color::White);

        let help = Line::from(help_options.iter().fold(
            Vec::new(),
            |mut acc, (key, description)| {
                if !acc.is_empty() {
                    acc.push(Span::raw(" | "));
                }
                if *key == "[/]" {
                    acc.push(Span::styled("[", blue_style));
                    acc.push(Span::styled("/", white_style));
                    acc.push(Span::styled("]", blue_style));
                } else {
                    acc.push(Span::styled(*key, blue_style));
                }
                acc.push(Span::raw(": "));
                acc.push(Span::styled(*description, light_blue_style));
                acc
            },
        ));

        let help = Paragraph::new(help);
        f.render_widget(help, content_help[1]);

        // ── Resources panel ──
        let resources_block = Block::default()
            .title("─Resources (nodes)")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(panel_border_style(
                self.dialog.as_ref(),
                self.focus,
                Focus::Resources,
            ));
        let resource_header = Row::new(vec![
            Cell::from(Line::from(vec![
                Span::styled("P", Style::default().add_modifier(Modifier::UNDERLINED)),
                Span::raw("artition"),
            ])),
            Cell::from(Line::from(vec![
                Span::styled("R", Style::default().add_modifier(Modifier::UNDERLINED)),
                Span::raw("unning"),
            ])),
            Cell::from(Line::from(vec![
                Span::styled("A", Style::default().add_modifier(Modifier::UNDERLINED)),
                Span::raw("vailable"),
            ])),
        ]);
        let resource_rows: Vec<Row> = if self.resources.is_empty() {
            vec![Row::new(vec![
                Cell::from(""),
                Cell::from(Line::from(Span::styled(
                    "No resource data",
                    Style::default().add_modifier(Modifier::DIM),
                ))),
                Cell::from(""),
            ])]
        } else {
            self.resources
                .iter()
                .map(|r| {
                    Row::new(vec![
                        Cell::from(r.partition.as_str()),
                        Cell::from(r.running_nodes.to_string()),
                        Cell::from(r.available_nodes.to_string()),
                    ])
                })
                .collect()
        };
        let resource_widths = [Constraint::Min(10), Constraint::Min(8), Constraint::Min(10)];
        let resource_table = Table::new(resource_rows, resource_widths)
            .header(resource_header)
            .block(resources_block)
            .column_spacing(1)
            .row_highlight_style(Style::default().bg(Color::Green).fg(Color::Black));
        f.render_stateful_widget(resource_table, top_row[0], &mut self.resource_table_state);
        self.resource_list_height = top_row[0].height.saturating_sub(3);
        self.resource_area = top_row[0];

        // Resources scrollbar
        let resource_viewport_height = usize::from(self.resource_list_height);
        let resource_content_height = self.resources.len().max(1); // always at least placeholder row
        if resource_viewport_height > 0 && resource_content_height > resource_viewport_height {
            let mut scrollbar_state = ScrollbarState::new(resource_content_height)
                .position(self.resource_table_state.offset())
                .viewport_content_length(resource_viewport_height);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));

            let scrollbar_area = Rect::new(
                self.resource_area.x.saturating_add(1),
                self.resource_area.y.saturating_add(2),
                self.resource_area.width.saturating_sub(2),
                self.resource_list_height,
            );
            f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }

        let visible_job_indices = self.visible_job_indices();
        let visible_jobs = visible_job_indices
            .iter()
            .map(|&index| &self.jobs[index])
            .collect::<Vec<_>>();

        let max_id_len = visible_jobs.iter().map(|j| j.id().len()).max().unwrap_or(0);
        let max_user_len = visible_jobs.iter().map(|j| j.user.len()).max().unwrap_or(0);
        let max_partition_len = visible_jobs
            .iter()
            .map(|j| j.partition.len())
            .max()
            .unwrap_or(0);
        let max_time_len = visible_jobs.iter().map(|j| j.time.len()).max().unwrap_or(0);
        let max_state_compact_len = visible_jobs
            .iter()
            .map(|j| j.state_compact.len())
            .max()
            .unwrap_or(0);
        let jobs_block = Block::default()
            .title(jobs_title(
                bottom_row[0].width,
                visible_job_indices.len(),
                self.jobs.len(),
                &self.active_filter,
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(panel_border_style(
                self.dialog.as_ref(),
                self.focus,
                Focus::Jobs,
            ));
        let header_style = Style::default();
        let job_header = Row::new(vec![
            sort_header_cell(
                "s",
                "t",
                header_style,
                self.sort_indicator(JobSortField::State),
            ),
            sort_header_cell(
                "p",
                "artition",
                header_style,
                self.sort_indicator(JobSortField::Partition),
            ),
            sort_header_cell(
                "i",
                "d",
                header_style,
                self.sort_indicator(JobSortField::Id),
            ),
            sort_header_cell(
                "n",
                "ame",
                header_style,
                self.sort_indicator(JobSortField::Name),
            ),
            sort_header_cell(
                "u",
                "ser",
                header_style,
                self.sort_indicator(JobSortField::User),
            ),
            sort_header_cell(
                "t",
                "ime",
                header_style,
                self.sort_indicator(JobSortField::Time),
            ),
        ]);
        let job_table_widths = [
            Constraint::Length((max_state_compact_len.max(3) as u16).min(4)),
            Constraint::Length((max_partition_len.max(10) as u16).min(12)),
            Constraint::Length((max_id_len.max(3) as u16).min(12)),
            Constraint::Min(8),
            Constraint::Length((max_user_len.max(5) as u16).min(12)),
            Constraint::Length((max_time_len.max(5) as u16).min(12)),
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
                        j.time.as_str(),
                        Style::default().fg(Color::Red),
                    ))),
                ])
            })
            .collect();
        let job_table = Table::new(jobs, job_table_widths)
            .header(job_header)
            .block(jobs_block)
            .column_spacing(1)
            .row_highlight_style(Style::default().bg(Color::Green).fg(Color::Black));
        f.render_stateful_widget(job_table, bottom_row[0], &mut self.job_list_state);
        self.job_list_height = bottom_row[0].height.saturating_sub(3);
        self.job_list_area = bottom_row[0];

        let job_list_viewport_height = usize::from(self.job_list_height);
        let job_list_content_height = visible_job_indices.len();
        let job_list_scroll_offset = self.job_list_state.offset();
        let job_list_rows_area = self.job_list_rows_area();

        if job_list_viewport_height > 0 && job_list_content_height > job_list_viewport_height {
            let mut job_list_scrollbar_state = ScrollbarState::new(job_list_content_height)
                .position(job_list_scroll_offset)
                .viewport_content_length(job_list_viewport_height);
            let job_list_scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));

            f.render_stateful_widget(
                job_list_scrollbar,
                job_list_rows_area,
                &mut job_list_scrollbar_state,
            );
        }

        let job_detail = self.selected_job();

        let job_detail = job_detail.map(|j| {
            let mut state_spans = vec![
                Span::styled("State  ", Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::raw(&j.state),
            ];
            if j.state == "PENDING" {
                state_spans.extend([
                    Span::styled(" Start ", Style::default().fg(Color::Yellow)),
                    Span::raw(&j.start_time),
                ]);
            }
            if let Some(s) = j.reason.as_deref() {
                state_spans.extend([
                    Span::styled(" Reason ", Style::default().fg(Color::Yellow)),
                    Span::raw(s),
                ]);
            }
            let state = Line::from(state_spans);
            let name = Line::from(vec![
                Span::styled("Name   ", Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::raw(&j.name),
            ]);
            let command = Line::from(vec![
                Span::styled("Command", Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::raw(&j.command),
            ]);
            let nodes = Line::from(vec![
                Span::styled("Nodes  ", Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::raw(&j.nodelist),
            ]);
            let tres = Line::from(vec![
                Span::styled("TRES   ", Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::raw(&j.tres),
            ]);
            let ui_stdout_text = match self.output_file_view {
                OutputFileView::Stdout => "stdout ",
                OutputFileView::Stderr => "stderr ",
            };
            let stdout = Line::from(vec![
                Span::styled(ui_stdout_text, Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::raw(
                    match self.output_file_view {
                        OutputFileView::Stdout => &j.stdout,
                        OutputFileView::Stderr => &j.stderr,
                    }
                    .as_ref()
                    .map(|p| p.to_str().unwrap_or_default())
                    .unwrap_or_default(),
                ),
            ]);

            Text::from(vec![state, name, command, nodes, tres, stdout])
        });
        let job_detail = Paragraph::new(job_detail.unwrap_or_default()).block(
            Block::default()
                .title("─Details")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(panel_border_style(
                    self.dialog.as_ref(),
                    self.focus,
                    Focus::Details,
                )),
        );
        f.render_widget(job_detail, top_row[1]);
        self.job_details_area = top_row[1];

        let log_area = bottom_row[1];
        self.job_output_area = log_area;
        let log_title = Line::from(vec![
            Span::raw("─"),
            Span::raw(match self.output_file_view {
                OutputFileView::Stdout => "stdout",
                OutputFileView::Stderr => "stderr",
            }),
            Span::styled(
                match self.job_output_anchor {
                    ScrollAnchor::Top if self.job_output_offset == 0 => "[T]".to_string(),
                    ScrollAnchor::Top => format!("[T+{}]", self.job_output_offset),
                    ScrollAnchor::Bottom if self.job_output_offset == 0 => "".to_string(),
                    ScrollAnchor::Bottom => format!("[B-{}]", self.job_output_offset),
                },
                Style::default().add_modifier(Modifier::DIM),
            ),
        ]);
        let log_block = Block::default()
            .title(log_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(panel_border_style(
                self.dialog.as_ref(),
                self.focus,
                Focus::Log,
            ));
        let log = match self.job_output.as_deref() {
            Ok(s) => Paragraph::new(fit_text(
                s,
                log_block.inner(log_area).height as usize,
                log_block.inner(log_area).width as usize,
                self.job_output_anchor,
                self.job_output_offset as usize,
                self.job_output_wrap,
            )),
            Err(e) => Paragraph::new(e.to_string())
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true }),
        }
        .block(log_block);

        f.render_widget(log, log_area);

        if let Some(dialog) = &self.dialog {
            match dialog {
                Dialog::CopyJobOutputDirectory { dir_url, dir_name } => {
                    let popup_width = min(f.area().width.saturating_sub(4).max(36), 72);
                    let content_width = popup_width.saturating_sub(4) as usize;
                    let rows = vec![
                        Line::from(vec![
                            Span::styled("[c]", Style::default().fg(Color::Yellow)),
                            Span::raw(" copy dir url"),
                        ]),
                        Line::from(Span::raw(truncate_with_ellipsis(dir_url, content_width))),
                        Line::default(),
                        Line::from(vec![
                            Span::styled("[d]", Style::default().fg(Color::Yellow)),
                            Span::raw(" copy directory name"),
                        ]),
                        Line::from(Span::raw(truncate_with_ellipsis(dir_name, content_width))),
                    ];
                    let popup_height = rows.len().saturating_add(2).min(u16::MAX as usize) as u16;

                    let dialog = Paragraph::new(Text::from(rows))
                        .style(Style::default().fg(Color::White))
                        .block(
                            Block::default()
                                .title("Copy:")
                                .borders(Borders::ALL)
                                .border_type(BorderType::Rounded)
                                .style(Style::default().fg(Color::Green)),
                        );

                    let area = centered_dialog_area(popup_width, popup_height, f.area());
                    f.render_widget(Clear, area);
                    f.render_widget(dialog, area);
                }
                Dialog::ConfirmCancelJob {
                    id, name, details, ..
                } => {
                    let popup_width = min(f.area().width.saturating_sub(4).max(36), 60);
                    let content_width = popup_width.saturating_sub(4) as usize;
                    let title = truncate_with_ellipsis("Cancel selected job?", content_width);
                    let mut rows = vec![
                        Line::from(Span::styled(
                            truncate_with_ellipsis(&format!("Job {id}"), content_width),
                            Style::default().add_modifier(Modifier::BOLD),
                        )),
                        Line::from(Span::raw(truncate_with_ellipsis(name, content_width))),
                    ];
                    rows.extend(details.iter().map(|detail| {
                        Line::from(Span::raw(truncate_with_ellipsis(detail, content_width)))
                    }));
                    rows.push(Line::default());
                    rows.push(Line::from(Span::styled(
                        "─".repeat(content_width),
                        Style::default().add_modifier(Modifier::DIM),
                    )));
                    rows.push(Line::from(vec![
                        Span::raw("            "),
                        Span::styled("[Y]es", Style::default().fg(Color::Black).bg(Color::Green)),
                        Span::raw("                 "),
                        Span::styled("(N)o", Style::default().fg(Color::White)),
                    ]));
                    let popup_height = rows.len().saturating_add(2).min(u16::MAX as usize) as u16;

                    let dialog = Paragraph::new(Text::from(rows))
                        .style(Style::default().fg(Color::White))
                        .block(
                            Block::default()
                                .title(Line::from(Span::styled(
                                    title,
                                    Style::default().add_modifier(Modifier::BOLD),
                                )))
                                .title_alignment(ratatui::layout::Alignment::Center)
                                .borders(Borders::ALL)
                                .border_type(BorderType::Rounded)
                                .style(Style::default().fg(Color::Green)),
                        );

                    let area = centered_dialog_area(popup_width, popup_height, f.area());
                    f.render_widget(Clear, area);
                    f.render_widget(dialog, area);
                }
                Dialog::EditTimeLimit { id, input } => {
                    let block = Block::default()
                        .title("─Time Limit")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(Color::Green));

                    let area = centered_dialog_area(DIALOG_WIDTH, 3, f.area());
                    let inner = block.inner(area);
                    let prompt_prefix = "Set time limit for job ";
                    let prompt_suffix = ": ";
                    let prompt_width = (prompt_prefix.chars().count()
                        + id.chars().count()
                        + prompt_suffix.chars().count())
                        as u16;
                    let available_width = inner.width.saturating_sub(prompt_width).max(1) as usize;
                    let scroll = input.visual_scroll(available_width);
                    let visible_value = input
                        .value()
                        .chars()
                        .skip(scroll)
                        .take(available_width)
                        .collect::<String>();
                    let dialog = Paragraph::new(Line::from(vec![
                        Span::raw(prompt_prefix),
                        Span::styled(id, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(prompt_suffix),
                        Span::styled(visible_value, Style::default().fg(Color::Blue)),
                    ]))
                    .style(Style::default().fg(Color::White))
                    .block(block);

                    f.render_widget(Clear, area);
                    f.render_widget(dialog, area);

                    let cursor_offset = input.visual_cursor().saturating_sub(scroll) as u16;
                    let cursor_x = inner
                        .x
                        .saturating_add(prompt_width)
                        .saturating_add(cursor_offset)
                        .min(inner.x.saturating_add(inner.width.saturating_sub(1)));
                    let cursor_y = inner.y;
                    f.set_cursor_position((cursor_x, cursor_y));
                }
                Dialog::FilterJobs { input } => {
                    let block = Block::default()
                        .title("Filter:")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(Color::Green));

                    let area = filter_popup_area(f.area());
                    let inner = block.inner(area);
                    let available_width = inner.width.max(1) as usize;
                    let scroll = input.visual_scroll(available_width);
                    let visible_value = input
                        .value()
                        .chars()
                        .skip(scroll)
                        .take(available_width)
                        .collect::<String>();
                    let dialog = Paragraph::new(Line::from(vec![Span::styled(
                        visible_value,
                        Style::default().fg(Color::Blue),
                    )]))
                    .style(Style::default().fg(Color::White))
                    .block(block);

                    f.render_widget(Clear, area);
                    f.render_widget(dialog, area);

                    let cursor_offset = input.visual_cursor().saturating_sub(scroll) as u16;
                    let cursor_x = inner
                        .x
                        .saturating_add(cursor_offset)
                        .min(inner.x.saturating_add(inner.width.saturating_sub(1)));
                    let cursor_y = inner.y;
                    f.set_cursor_position((cursor_x, cursor_y));
                }
                Dialog::CommandError { command, output } => {
                    let dialog_text = format!("Command: {command}\n\n{output}");
                    let lines = dialog_text
                        .lines()
                        .count()
                        .saturating_add(2)
                        .min(u16::MAX as usize) as u16;
                    let dialog = Paragraph::new(dialog_text)
                        .style(Style::default().fg(Color::White))
                        .wrap(Wrap { trim: false })
                        .block(
                            Block::default()
                                .title("─Command Error")
                                .borders(Borders::ALL)
                                .border_type(BorderType::Rounded)
                                .style(Style::default().fg(Color::Red)),
                        );

                    let area = centered_dialog_area(DIALOG_WIDTH, lines, f.area());
                    f.render_widget(Clear, area);
                    f.render_widget(dialog, area);
                }
            }
        }
    }
}

fn panel_border_style(dialog: Option<&Dialog>, focus: Focus, panel: Focus) -> Style {
    if dialog.is_some() {
        Style::default()
    } else if focus == panel {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    }
}

fn centered_dialog_area(width: u16, lines: u16, viewport: Rect) -> Rect {
    let dialog_width = min(width, viewport.width);
    let dialog_height = min(lines, viewport.height);
    let dialog_x = viewport.x + viewport.width.saturating_sub(dialog_width) / 2;
    let dialog_y = viewport.y + viewport.height.saturating_sub(dialog_height) / 2;

    Rect::new(dialog_x, dialog_y, dialog_width, dialog_height)
}

fn filter_popup_area(viewport: Rect) -> Rect {
    let max_width = viewport.width.saturating_sub(4).max(1);
    let min_width = min(30, max_width);
    let preferred_width = min(max_width, ((viewport.width as usize * 3) / 5) as u16);
    let centered = centered_dialog_area(
        preferred_width.max(min_width),
        min(3, viewport.height),
        viewport,
    );

    Rect::new(
        centered.x,
        centered.y.saturating_sub(1),
        centered.width,
        centered.height,
    )
}

fn jobs_title(width: u16, visible_count: usize, total_count: usize, active_filter: &str) -> String {
    let title = if active_filter.is_empty() {
        format!("─Jobs ({total_count})")
    } else {
        format!("─Jobs ({visible_count}/{total_count}) filter: {active_filter}")
    };

    truncate_with_ellipsis(&title, width.saturating_sub(4) as usize)
}

fn truncate_with_ellipsis(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return value.to_string();
    }
    if max_chars == 0 {
        return String::new();
    }
    if max_chars == 1 {
        return "…".to_string();
    }

    chars
        .into_iter()
        .take(max_chars.saturating_sub(1))
        .chain(once('…'))
        .collect()
}

pub(super) fn chunked_string(s: &str, first_chunk_size: usize, chunk_size: usize) -> Vec<&str> {
    let stepped_indices = s
        .char_indices()
        .map(|(i, _)| i)
        .enumerate()
        .filter(|&(i, _)| {
            if i > first_chunk_size {
                chunk_size > 0 && (i - first_chunk_size).is_multiple_of(chunk_size)
            } else {
                i == 0 || i == first_chunk_size
            }
        })
        .map(|(_, e)| e)
        .collect::<Vec<_>>();
    let windows = stepped_indices.windows(2).collect::<Vec<_>>();

    let iter = windows.iter().map(|w| &s[w[0]..w[1]]);
    let last_index = *stepped_indices.last().unwrap_or(&0);
    iter.chain(once(&s[last_index..])).collect()
}

fn fit_text(
    s: &'_ str,
    lines: usize,
    cols: usize,
    anchor: ScrollAnchor,
    offset: usize,
    wrap: bool,
) -> Text<'_> {
    let s = s.rsplit_once(['\r', '\n']).map_or(s, |(p, _)| p);
    let l = s.lines().flat_map(|l| l.split('\r'));
    let iter = match anchor {
        ScrollAnchor::Top => Either::Left(l),
        ScrollAnchor::Bottom => Either::Right(l.rev()),
    };
    let iter = iter
        .skip(offset)
        .flat_map(|l| {
            let iter = if wrap {
                Either::Left(
                    chunked_string(l, cols, cols.saturating_sub(2))
                        .into_iter()
                        .enumerate()
                        .map(|(i, l)| {
                            if i == 0 {
                                Line::raw(l.chars().take(cols).collect::<String>())
                            } else {
                                Line::default().spans(vec![
                                    Span::styled(
                                        "↪ ",
                                        Style::default().add_modifier(Modifier::DIM),
                                    ),
                                    Span::raw(
                                        l.chars().take(cols.saturating_sub(2)).collect::<String>(),
                                    ),
                                ])
                            }
                        }),
                )
            } else {
                match l.chars().nth(cols) {
                    Some(_) => Either::Right(once(Line::default().spans(vec![
                        Span::raw(l.chars().take(cols.saturating_sub(1)).collect::<String>()),
                        Span::styled("…", Style::default().add_modifier(Modifier::DIM)),
                    ]))),
                    None => {
                        Either::Right(once(Line::raw(l.chars().take(cols).collect::<String>())))
                    }
                }
            };
            match anchor {
                ScrollAnchor::Top => Either::Left(iter),
                ScrollAnchor::Bottom => Either::Right(iter.rev()),
            }
        })
        .take(lines);

    match anchor {
        ScrollAnchor::Top => Text::from(iter.collect::<Vec<_>>()),
        ScrollAnchor::Bottom => Text::from(
            iter.collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>(),
        ),
    }
}

fn sort_header_cell<'a>(
    first: &'a str,
    rest: &'a str,
    style: Style,
    indicator: &'static str,
) -> Cell<'a> {
    Cell::from(Line::from(vec![
        Span::styled(first, style.add_modifier(Modifier::UNDERLINED)),
        Span::styled(rest, style),
        Span::styled(indicator, style),
    ]))
}
