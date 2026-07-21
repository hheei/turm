use super::*;

impl App {
    pub(super) fn render_resources(&mut self, f: &mut Frame, area: Rect) {
        let resources_block = Block::default()
            .title(Line::from(vec![
                Span::styled("─", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    " Resources ",
                    Style::default().fg(if self.focus == Focus::Resources {
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
        let resource_header = Row::new(vec![
            Cell::from("Partition"),
            Cell::from("Used"),
            Cell::from("Avail"),
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
                        Cell::from(format!("{}({})", r.running_nodes, r.group_used_nodes)),
                        Cell::from(r.available_nodes.to_string()),
                    ])
                    .style(if r.available_nodes == 0 {
                        Style::default().add_modifier(Modifier::DIM)
                    } else {
                        Style::default()
                    })
                })
                .collect()
        };
        let resource_widths = [Constraint::Min(10), Constraint::Min(8), Constraint::Min(5)];
        let resource_table = Table::new(resource_rows, resource_widths)
            .header(resource_header)
            .block(resources_block)
            .column_spacing(1)
            .row_highlight_style(if self.focus == Focus::Resources {
                Style::default().bg(Color::Green).fg(Color::Black)
            } else {
                Style::default()
            });
        f.render_stateful_widget(resource_table, area, &mut self.resource_table_state);
        self.resource_list_height = area.height.saturating_sub(3);
        self.resource_area = area;

        let resource_viewport_height = usize::from(self.resource_list_height);
        let resource_content_height = self.resources.len().max(1);
        if resource_viewport_height > 0 && resource_content_height > resource_viewport_height {
            let max_offset = resource_content_height.saturating_sub(resource_viewport_height);
            let mut scrollbar_state = ScrollbarState::new(max_offset.saturating_add(1))
                .position(self.resource_table_state.offset())
                .viewport_content_length(resource_viewport_height);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalLeft)
                .thumb_symbol(VERTICAL_SCROLLBAR_THUMB)
                .track_symbol(Some(VERTICAL_SCROLLBAR_TRACK))
                .begin_symbol(None)
                .end_symbol(None);
            let scrollbar_area = Rect::new(
                self.resource_area.x.saturating_add(1),
                self.resource_area.y.saturating_add(2),
                self.resource_area.width.saturating_sub(2),
                self.resource_list_height,
            );
            f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }
    }
}
