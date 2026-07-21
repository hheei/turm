use super::*;

impl App {
    pub(super) fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_options = match &self.dialog {
            Some(Dialog::FilterJobs { .. }) => {
                vec![("enter", "apply"), ("esc", "cancel"), ("ctrl+u", "clear")]
            }
            Some(Dialog::CopyJobOutputDirectory { .. }) => {
                vec![("c", "dir-name"), ("d", "dir-url"), ("⎋", "cancel")]
            }
            Some(Dialog::ConfirmCancelJob { .. }) => {
                vec![("←/→", "choose"), ("enter", "confirm"), ("esc", "close")]
            }
            Some(Dialog::EditJobName { .. }) => {
                vec![("enter", "apply"), ("esc", "cancel")]
            }
            _ => Vec::new(),
        };

        if self.dialog.is_none() {
            let key_style = Style::default().fg(Color::Blue);
            let text_style = Style::default().fg(Color::DarkGray);
            let optional = Line::from(vec![
                Span::styled("⌃r", key_style),
                Span::styled(" rename · ", text_style),
                Span::styled("⌃d", key_style),
                Span::styled(" cancel · ", text_style),
                Span::styled("⌃t", key_style),
                Span::styled(" set time · ", text_style),
                Span::styled("⌃c", key_style),
                Span::styled(" path", text_style),
            ]);
            let enter_action = match self.output_panel_mode {
                OutputPanelMode::Stdout | OutputPanelMode::Stderr => "edit",
                OutputPanelMode::Workdir | OutputPanelMode::Collapsed => "cd",
            };
            let always = Line::from(vec![
                Span::styled("⇥", key_style),
                Span::styled(" toggle · ", text_style),
                Span::styled("↵", key_style),
                Span::styled(format!(" {enter_action} · "), text_style),
                Span::styled("d", key_style),
                Span::styled(" detail · ", text_style),
                Span::styled("f", key_style),
                Span::styled(" filter · ", text_style),
                Span::styled("q", key_style),
                Span::styled(" exit", text_style),
            ]);
            let text = if area.height > 1 {
                Text::from(vec![optional, always])
            } else {
                Text::from(always)
            };
            f.render_widget(Paragraph::new(text), area);
            return;
        }

        let blue_style = Style::default().fg(Color::Blue);
        let light_blue_style = Style::default().fg(Color::LightBlue);

        let help = Line::from(help_options.iter().fold(
            Vec::new(),
            |mut acc, (key, description)| {
                if !acc.is_empty() {
                    acc.push(Span::raw(" | "));
                }
                acc.push(Span::styled(*key, blue_style));
                acc.push(Span::raw(": "));
                acc.push(Span::styled(*description, light_blue_style));
                acc
            },
        ));
        f.render_widget(Paragraph::new(help), area);
    }
}
