use super::*;

pub(super) const VERTICAL_SCROLLBAR_THUMB: &str = "┃";
pub(super) const VERTICAL_SCROLLBAR_TRACK: &str = "│";
pub(super) const OUTPUT_HORIZONTAL_SCROLLBAR_THUMB: &str = "━";
pub(super) const OUTPUT_HORIZONTAL_SCROLLBAR_TRACK: &str = "─";

struct UiAreas {
    help: Rect,
    resources: Rect,
    details: Rect,
    jobs: Rect,
    output: Option<Rect>,
}

impl UiAreas {
    fn new(area: Rect, output_collapsed: bool, details_visible: bool) -> Self {
        let help_height = if area.height > 20 { 2 } else { 1 };
        let content_help = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(help_height)])
            .split(area);
        let content = content_help[0];
        let output_can_expand = content.height >= 27;
        let (top, output) = if output_collapsed || !output_can_expand {
            (content, None)
        } else {
            let top_height = content.height.saturating_sub(10).clamp(23, 31);
            let panels = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(top_height), Constraint::Min(4)])
                .split(content);
            (panels[0], Some(panels[1]))
        };
        let top_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(top);
        let job_container = Rect::new(
            top_row[1].x.saturating_sub(1),
            top_row[1].y,
            top_row[1].width.saturating_add(1),
            top_row[1].height,
        );
        let details = if details_visible && job_container.width >= 28 && job_container.height >= 6 {
            let width = (job_container.width.saturating_mul(52) / 100)
                .clamp(30, job_container.width.saturating_sub(2));
            let height = job_container.height.saturating_sub(3).min(10);
            Rect::new(
                job_container
                    .right()
                    .saturating_sub(width)
                    .saturating_sub(1),
                job_container
                    .bottom()
                    .saturating_sub(height)
                    .saturating_sub(1),
                width,
                height,
            )
        } else {
            Rect::default()
        };

        Self {
            help: content_help[1],
            resources: top_row[0],
            details,
            jobs: job_container,
            output,
        }
    }
}

impl App {
    pub(super) fn ui(&mut self, frame: &mut Frame) {
        let areas = UiAreas::new(
            frame.area(),
            self.output_panel_mode.is_collapsed(),
            self.details_visible,
        );
        self.output_can_expand = frame.area().height.saturating_sub(areas.help.height) >= 27;
        if !self.output_can_expand && self.focus == Focus::Log {
            self.focus = Focus::Jobs;
        }
        self.render_help(frame, areas.help);
        self.render_resources(frame, areas.resources);
        self.render_jobs(frame, areas.jobs);
        self.render_top_panel_seam(frame, areas.jobs);
        if self.details_visible {
            self.render_details(frame, areas.details);
        } else {
            self.job_details_area = Rect::default();
        }
        self.render_output(frame, areas.output);
        self.render_dialog(frame);
        if self.dialog.is_none() {
            self.render_mouse_selection(frame);
            self.render_clipboard_notice(frame, areas.help);
        }
        self.screen_buffer = Some(frame.buffer_mut().clone());
    }

    fn render_mouse_selection(&self, frame: &mut Frame) {
        let Some(selection) = self.mouse_selection else {
            return;
        };
        if !selection.dragged {
            return;
        }
        let (top, bottom) = self.mouse_selection_y_bounds(selection);
        let style = Style::default().bg(Color::Blue).fg(Color::White);
        for y in top..=bottom {
            let Some((left, right)) = self.mouse_selection_row_bounds(selection, y) else {
                continue;
            };
            for x in left..=right {
                frame.buffer_mut()[(x, y)].set_style(style);
            }
        }
    }

    fn render_clipboard_notice(&self, frame: &mut Frame, footer: Rect) {
        if self
            .clipboard_notice_until
            .is_none_or(|until| until <= Instant::now())
        {
            return;
        }

        let width = 23.min(frame.area().width);
        let area = Rect::new(
            frame.area().x + frame.area().width.saturating_sub(width) / 2,
            footer.y.saturating_sub(2),
            width,
            3.min(frame.area().height),
        );
        let notice = Paragraph::new("Copied to clipboard").block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1)),
        );
        frame.render_widget(Clear, area);
        frame.render_widget(notice, area);
    }

    fn render_top_panel_seam(&self, frame: &mut Frame, details: Rect) {
        if details.is_empty() {
            return;
        }

        let style = Style::default().fg(Color::DarkGray);
        let seam_x = details.x;
        let bottom_y = details.y + details.height.saturating_sub(1);
        frame.buffer_mut().set_string(seam_x, details.y, "┬", style);
        for y in details.y.saturating_add(1)..bottom_y {
            frame.buffer_mut().set_string(seam_x, y, "│", style);
        }
        frame.buffer_mut().set_string(seam_x, bottom_y, "┴", style);
    }
}

mod details;
mod dialogs;
mod help;
mod helpers;
mod jobs;
mod output;
mod resources;

use helpers::*;
pub(super) use helpers::{chunked_string, job_output_line_count};
