use super::*;

impl App {
    pub(super) fn render_dialog(&self, f: &mut Frame) {
        if let Some(dialog) = &self.dialog {
            match dialog {
                Dialog::CopyJobOutputDirectory { dir_url, dir_name } => {
                    let popup_width = min(f.area().width.saturating_sub(4).max(36), 72);
                    let content_width = popup_width.saturating_sub(6) as usize;
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
                                .title(Line::from(vec![
                                    Span::styled("─", Style::default().fg(Color::Green)),
                                    Span::styled(" Copy ", Style::default().fg(Color::Green)),
                                    Span::styled("─", Style::default().fg(Color::Green)),
                                ]))
                                .borders(Borders::ALL)
                                .border_type(BorderType::Rounded)
                                .padding(Padding::horizontal(1))
                                .style(Style::default().fg(Color::Green)),
                        );
                    let area = centered_dialog_area(popup_width, popup_height, f.area());
                    f.render_widget(Clear, area);
                    f.render_widget(dialog, area);
                }
                Dialog::ConfirmCancelJob {
                    id,
                    name,
                    details,
                    selected,
                    ..
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
                    let no_style = if *selected == ConfirmCancelChoice::No {
                        Style::default().fg(Color::Black).bg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let yes_style = if *selected == ConfirmCancelChoice::Yes {
                        Style::default().fg(Color::Black).bg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    rows.push(Line::from(vec![
                        Span::raw("            "),
                        Span::styled("[N]o", no_style),
                        Span::raw("                 "),
                        Span::styled("[Y]es", yes_style),
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
                        Span::raw(visible_value),
                    ]))
                    .style(Style::default().fg(Color::White))
                    .block(block);
                    f.render_widget(Clear, area);
                    f.render_widget(dialog, area);
                    f.set_cursor_position((
                        inner.x.saturating_add(prompt_width)
                            + input.visual_cursor().saturating_sub(scroll) as u16,
                        inner.y,
                    ));
                }
                Dialog::EditJobName { id, input } => {
                    let block = Block::default()
                        .title("─Rename Job")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(Style::default().fg(Color::Green));
                    let area = centered_dialog_area(DIALOG_WIDTH, 3, f.area());
                    let inner = block.inner(area);
                    let prompt_prefix = "Set name for job ";
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
                        Span::raw(visible_value),
                    ]))
                    .style(Style::default().fg(Color::White))
                    .block(block);
                    f.render_widget(Clear, area);
                    f.render_widget(dialog, area);
                    f.set_cursor_position((
                        inner.x.saturating_add(prompt_width)
                            + input.visual_cursor().saturating_sub(scroll) as u16,
                        inner.y,
                    ));
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
                    let dialog = Paragraph::new(visible_value)
                        .style(Style::default().fg(Color::White))
                        .block(block);
                    f.render_widget(Clear, area);
                    f.render_widget(dialog, area);
                    f.set_cursor_position((
                        inner.x + input.visual_cursor().saturating_sub(scroll) as u16,
                        inner.y,
                    ));
                }
                Dialog::CommandError { command, output } => {
                    let dialog_text = format!("Command: {command}\n\n{output}");
                    let lines = dialog_text.lines().count().min(10) as u16 + 2;
                    let area = centered_dialog_area(DIALOG_WIDTH, lines, f.area());
                    let dialog = Paragraph::new(dialog_text)
                        .style(Style::default().fg(Color::White))
                        .wrap(Wrap { trim: true })
                        .block(
                            Block::default()
                                .title("Command failed")
                                .borders(Borders::ALL)
                                .border_type(BorderType::Rounded)
                                .style(Style::default().fg(Color::Green)),
                        );
                    f.render_widget(Clear, area);
                    f.render_widget(dialog, area);
                }
            }
        }
    }
}
