#![allow(dead_code, unused_imports)]

pub use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
pub use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Modifier},
};
pub use std::path::PathBuf;
pub use tui_input::Input;
pub use turm::test_support::{
    AppDriver, TestJobSortField as JobSortField, TestSortDirection as SortDirection,
    TestWorkdirEntry as WorkdirEntry, TestWorkdirEntryKind as WorkdirEntryKind,
    cancellation_action as cancel_confirmation_action, chunk_string as chunked_string,
    validate_time_limit as validated_time_limit, watched_path as watched_output_path,
};
pub use turm::{
    AppExit, AppMessage, ConfirmCancelChoice, Dialog, Focus, Job, OutputPanelMode, ScrollAnchor,
};

pub const VERTICAL_SCROLLBAR_THUMB: &str = "┃";
pub const OUTPUT_HORIZONTAL_SCROLLBAR_THUMB: &str = "━";

pub fn test_job(index: usize) -> Job {
    turm::test_support::test_job(index)
}

pub fn test_app(job_count: usize, selected: Option<usize>) -> AppDriver {
    AppDriver::new(job_count, selected)
}

pub fn draw_app(app: &mut AppDriver, width: u16, height: u16) -> Buffer {
    app.render(width, height)
}

pub fn row_text(buffer: &Buffer, area: Rect, y: u16) -> String {
    (area.x..area.x.saturating_add(area.width))
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

pub fn jobs_area_symbols(buffer: &Buffer, area: Rect) -> Vec<String> {
    (area.y..area.y.saturating_add(area.height))
        .flat_map(|y| {
            (area.x..area.x.saturating_add(area.width))
                .map(move |x| buffer[(x, y)].symbol().to_string())
        })
        .collect()
}

pub fn symbol_columns(buffer: &Buffer, area: Rect, symbol: &str) -> Vec<u16> {
    (area.x..area.x.saturating_add(area.width))
        .filter(|&x| {
            (area.y..area.y.saturating_add(area.height)).any(|y| buffer[(x, y)].symbol() == symbol)
        })
        .collect()
}

pub fn symbol_rows(buffer: &Buffer, area: Rect, symbol: &str) -> Vec<u16> {
    (area.y..area.y.saturating_add(area.height))
        .filter(|&y| {
            (area.x..area.x.saturating_add(area.width)).any(|x| buffer[(x, y)].symbol() == symbol)
        })
        .collect()
}

pub fn scrollbar_thumb_top(buffer: &Buffer, area: Rect) -> Option<u16> {
    symbol_rows(buffer, area, VERTICAL_SCROLLBAR_THUMB)
        .into_iter()
        .next()
}

pub fn output_inner_test_area(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(2),
        area.y.saturating_add(1),
        area.width.saturating_sub(3),
        area.height.saturating_sub(2),
    )
}

pub fn output_horizontal_scrollbar_area(app: &AppDriver) -> Option<Rect> {
    let layout = app.layout();
    layout.show_horizontal.then(|| {
        let inner = output_inner_test_area(app.job_output_area());
        Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width.saturating_sub(u16::from(layout.show_vertical)),
            1,
        )
    })
}

pub fn output_vertical_scrollbar_area(app: &AppDriver) -> Option<Rect> {
    let layout = app.layout();
    layout.show_vertical.then(|| {
        let inner = output_inner_test_area(app.job_output_area());
        Rect::new(
            inner.right().saturating_sub(1),
            inner.y,
            1,
            inner
                .height
                .saturating_sub(u16::from(layout.show_horizontal)),
        )
    })
}

pub fn app_with_jobs(jobs: Vec<Job>, selected: Option<usize>) -> AppDriver {
    AppDriver::with_jobs(jobs, selected)
}

pub fn visible_job_ids(app: &AppDriver) -> Vec<String> {
    app.visible_job_ids()
}

pub fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub fn buffer_text(buffer: &Buffer, width: u16, height: u16) -> String {
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn filter_test_jobs() -> Vec<Job> {
    let mut alpha = test_job(0);
    alpha.job_id = "101512".to_string();
    alpha.array_id = "101512".to_string();
    alpha.name = "vasp-alpha".to_string();
    alpha.user = "chlo".to_string();
    alpha.partition = "gpu".to_string();
    alpha.state = "RUNNING".to_string();
    alpha.state_compact = "R".to_string();
    alpha.time = "7:00".to_string();

    let mut beta = test_job(1);
    beta.job_id = "202000".to_string();
    beta.array_id = "202000".to_string();
    beta.name = "relax-beta".to_string();
    beta.user = "alex".to_string();
    beta.partition = "debug".to_string();
    beta.state = "PENDING".to_string();
    beta.state_compact = "PD".to_string();
    beta.time = "00:10:00".to_string();

    let mut gamma = test_job(2);
    gamma.job_id = "303333".to_string();
    gamma.array_id = "303333".to_string();
    gamma.name = "analysis-gamma".to_string();
    gamma.user = "chlo".to_string();
    gamma.partition = "cpu".to_string();
    gamma.state = "COMPLETED".to_string();
    gamma.state_compact = "CG".to_string();
    gamma.time = "01:23:00".to_string();

    vec![alpha, beta, gamma]
}

pub fn open_filter(app: &mut AppDriver) {
    app.handle(AppMessage::Key(key('f')));
}

pub fn type_in_filter(app: &mut AppDriver, value: &str) {
    for ch in value.chars() {
        app.handle(AppMessage::Key(key(ch)));
    }
}

pub fn key(char_key: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(char_key), KeyModifiers::NONE)
}

pub fn special_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub fn key_with_modifiers(char_key: char, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(char_key), modifiers)
}

pub fn filtered_jobs_app() -> AppDriver {
    let mut app = app_with_jobs(filter_test_jobs(), Some(0));
    app.apply_job_filter("id:101512");
    app
}

pub fn app_with_output_lines(lines: &[&str]) -> AppDriver {
    let mut app = test_app(1, Some(0));
    app.set_output_mode(OutputPanelMode::Stdout);
    app.set_job_output(lines.join("\n"));
    app
}

pub fn copyable_jobs_app() -> AppDriver {
    let mut app = filtered_jobs_app();
    app.jobs_mut()[0].stdout = Some(PathBuf::from("/scratch/chlo/vasp-alpha/stdout.log"));
    app.jobs_mut()[0].stderr = Some(PathBuf::from("/scratch/chlo/vasp-alpha/stderr.log"));
    app
}

pub fn copyable_jobs_app_with_output_path(path: &str) -> AppDriver {
    let mut app = filtered_jobs_app();
    app.jobs_mut()[0].stdout = Some(PathBuf::from(path));
    app.jobs_mut()[0].stderr = Some(PathBuf::from(path));
    app
}

pub fn details_area(app: &AppDriver) -> Rect {
    app.job_details_area()
}

pub fn border_fg(buffer: &Buffer, area: Rect) -> Option<Color> {
    buffer[(area.x, area.y)].style().fg
}
