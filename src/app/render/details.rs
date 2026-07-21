use super::*;

impl App {
    pub(super) fn render_details(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("─ Details ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .padding(Padding::horizontal(1))
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        let job_detail = self.selected_job().map(|j| {
            let mut lines = Vec::new();
            let mut groups = Vec::new();
            let mut group = 0;
            append_detail(
                &mut lines,
                &mut groups,
                group,
                "State",
                &j.state,
                inner.width,
            );
            if j.state == "PENDING" {
                group += 1;
                append_detail(
                    &mut lines,
                    &mut groups,
                    group,
                    "Start",
                    &j.start_time,
                    inner.width,
                );
            }
            if let Some(s) = j.reason.as_deref() {
                group += 1;
                append_detail(&mut lines, &mut groups, group, "Reason", s, inner.width);
            }
            for (label, value) in [
                ("Name", j.name.as_str()),
                ("Command", j.command.as_str()),
                ("Nodes", j.nodelist.as_str()),
                ("TRES", j.tres.as_str()),
            ] {
                group += 1;
                append_detail(&mut lines, &mut groups, group, label, value, inner.width);
            }
            let output_label = match self.output_panel_mode {
                OutputPanelMode::Stdout => "stdout ",
                OutputPanelMode::Stderr => "stderr ",
                OutputPanelMode::Workdir => "workdir",
                OutputPanelMode::Collapsed => "stdout ",
            };
            let output_value = match self.output_panel_mode {
                OutputPanelMode::Stdout => j.stdout.as_ref(),
                OutputPanelMode::Stderr => j.stderr.as_ref(),
                OutputPanelMode::Workdir => self.workdir_path.as_ref(),
                OutputPanelMode::Collapsed => j.stdout.as_ref(),
            }
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
            group += 1;
            append_detail(
                &mut lines,
                &mut groups,
                group,
                output_label.trim(),
                &output_value,
                inner.width,
            );
            (Text::from(lines), groups)
        });
        self.details_selection_rows.clear();
        let text = if let Some((text, groups)) = job_detail {
            let left = inner.x;
            let value_x = left.saturating_add(9);
            let right = inner.right().saturating_sub(1);
            self.details_selection_rows = groups
                .into_iter()
                .enumerate()
                .map(|(index, group)| DetailsSelectionRow {
                    y: inner.y.saturating_add(index as u16),
                    group,
                    left,
                    value_x,
                    right,
                })
                .collect();
            text
        } else {
            Text::default()
        };
        let job_detail = Paragraph::new(text).block(block);
        f.render_widget(job_detail, area);
        self.job_details_area = area;
    }
}

fn append_detail(
    lines: &mut Vec<Line<'static>>,
    groups: &mut Vec<usize>,
    group: usize,
    label: &str,
    value: &str,
    width: u16,
) {
    let label_width = 8usize;
    let value_width = usize::from(width).saturating_sub(label_width + 1).max(1);
    let chunks = if label == "TRES" {
        tres_chunks(value, value_width)
    } else {
        value
            .chars()
            .collect::<Vec<_>>()
            .chunks(value_width)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<_>>()
    };
    let chunks = if chunks.is_empty() {
        vec![String::new()]
    } else {
        chunks
    };
    let detail = chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            let prefix = if index == 0 {
                format!("{label:<label_width$} ")
            } else {
                " ".repeat(label_width + 1)
            };
            Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::raw(chunk),
            ])
        })
        .collect::<Vec<_>>();
    groups.extend(std::iter::repeat_n(group, detail.len()));
    lines.extend(detail);
}

fn tres_chunks(value: &str, width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut line = String::new();
    for item in value.split(',').filter(|item| !item.is_empty()) {
        let fits = line.is_empty() || line.chars().count() + 1 + item.chars().count() <= width;
        if !fits {
            chunks.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(',');
        }
        line.push_str(item);
    }
    if !line.is_empty() {
        chunks.push(line);
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::tres_chunks;

    #[test]
    fn tres_wrap_keeps_each_key_value_pair_together() {
        let chunks = tres_chunks("billing=128,cpu=128,mem=962.50G,node=4", 19);
        assert_eq!(chunks, ["billing=128,cpu=128", "mem=962.50G,node=4"]);
        assert!(
            chunks
                .iter()
                .all(|line| line.split(',').all(|item| item.contains('=')))
        );
    }
}
