use super::*;
use super::commands::validated_time_limit;
use super::render::chunked_string;
use crossbeam::channel::unbounded;
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    style::{Color, Modifier},
};

fn test_job(index: usize) -> Job {
    Job {
        job_id: format!("{}", 1000 + index),
        array_id: format!("{}", 1000 + index),
        array_step: None,
        name: format!("job-{index}"),
        state: "RUNNING".to_string(),
        state_compact: "R".to_string(),
        reason: None,
        user: "chlo".to_string(),
        time: format!("00:{:02}:00", index % 60),
        time_limit: "01:00:00".to_string(),
        start_time: "N/A".to_string(),
        tres: "cpu=1".to_string(),
        partition: "debug".to_string(),
        nodelist: "node-01".to_string(),
        stdout: None,
        stderr: None,
        command: format!("run-job-{index}"),
    }
}

fn test_app(job_count: usize, selected: Option<usize>) -> App {
    let (app_sender, receiver) = unbounded();
    let (_input_sender, input_receiver) = unbounded();

    App {
        focus: Focus::Jobs,
        dialog: None,
        jobs: (0..job_count).map(test_job).collect(),
        job_list_state: TableState::new().with_selected(selected),
        job_sort_field: JobSortField::Id,
        job_sort_direction: SortDirection::Desc,
        job_output: Ok(String::new()),
        job_output_anchor: ScrollAnchor::Bottom,
        job_output_offset: 0,
        job_output_wrap: false,
        _job_watcher: JobWatcherHandle {},
        job_output_watcher: FileWatcherHandle::new(app_sender, Duration::from_secs(60)),
        receiver,
        input_receiver,
        output_file_view: OutputFileView::default(),
        job_list_height: 0,
        job_list_area: Rect::default(),
        job_output_area: Rect::default(),
        pending_input_event: None,
    }
}

fn draw_app(app: &mut App, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.ui(f)).unwrap();
    terminal.backend().buffer().clone()
}

fn row_text(buffer: &Buffer, area: Rect, y: u16) -> String {
    (area.x..area.x.saturating_add(area.width))
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

fn jobs_area_symbols(buffer: &Buffer, area: Rect) -> Vec<String> {
    (area.y..area.y.saturating_add(area.height))
        .flat_map(|y| {
            (area.x..area.x.saturating_add(area.width))
                .map(move |x| buffer[(x, y)].symbol().to_string())
        })
        .collect()
}

fn scrollbar_thumb_top(buffer: &Buffer, area: Rect) -> Option<u16> {
    (area.y..area.y.saturating_add(area.height)).find(|&y| {
        (area.x..area.x.saturating_add(area.width)).any(|x| buffer[(x, y)].symbol() == "█")
    })
}

fn app_with_jobs(jobs: Vec<Job>, selected: Option<usize>) -> App {
    let mut app = test_app(0, selected);
    app.jobs = jobs;
    app
}

fn key(char_key: char) -> KeyEvent {
    KeyEvent::new(
        KeyCode::Char(char_key),
        crossterm::event::KeyModifiers::NONE,
    )
}

fn key_with_modifiers(char_key: char, modifiers: crossterm::event::KeyModifiers) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(char_key), modifiers)
}

#[test]
fn jobs_table_header_renders_with_underlined_shortcuts() {
    let mut app = test_app(2, Some(0));
    let buffer = draw_app(&mut app, 120, 12);
    let header_y = app.job_list_area.y.saturating_add(1);
    let header_text = row_text(&buffer, app.job_list_area, header_y);

    for label in ["st", "partition", "id", "name", "user", "time"] {
        assert!(
            header_text.contains(label),
            "missing header label {label}: {header_text}"
        );
    }

    let underlined_header_symbols = (app.job_list_area.x
        ..app.job_list_area.x.saturating_add(app.job_list_area.width))
        .filter_map(|x| {
            let cell = &buffer[(x, header_y)];
            cell.style()
                .add_modifier
                .contains(Modifier::UNDERLINED)
                .then(|| cell.symbol().to_string())
        })
        .collect::<Vec<_>>();

    assert_eq!(
        underlined_header_symbols,
        vec!["s", "p", "i", "n", "u", "t"]
    );
}

#[test]
fn jobs_panel_uses_a_40_percent_layout_split() {
    let mut app = test_app(3, Some(0));
    let _ = draw_app(&mut app, 100, 12);

    assert_eq!(app.job_list_area.width, 40);
    assert_eq!(app.job_output_area.x, 40);
    assert_eq!(app.job_output_area.width, 60);
}

#[test]
fn sort_keys_toggle_direction_and_switch_fields() {
    let mut app = test_app(3, Some(0));

    app.handle(AppMessage::Key(key('n')));
    assert_eq!(app.job_sort_field, JobSortField::Name);
    assert_eq!(app.job_sort_direction, SortDirection::Desc);

    app.handle(AppMessage::Key(key('n')));
    assert_eq!(app.job_sort_field, JobSortField::Name);
    assert_eq!(app.job_sort_direction, SortDirection::Asc);

    app.handle(AppMessage::Key(key('p')));
    assert_eq!(app.job_sort_field, JobSortField::Partition);
    assert_eq!(app.job_sort_direction, SortDirection::Desc);
}

#[test]
fn header_renders_active_sort_indicator() {
    let mut app = test_app(2, Some(0));
    let buffer = draw_app(&mut app, 120, 12);
    let header_y = app.job_list_area.y.saturating_add(1);
    let header_text = row_text(&buffer, app.job_list_area, header_y);
    assert!(header_text.contains("id▼"), "header was {header_text}");

    app.handle(AppMessage::Key(key('n')));
    app.handle(AppMessage::Key(key('n')));
    let buffer = draw_app(&mut app, 120, 12);
    let header_text = row_text(&buffer, app.job_list_area, header_y);
    assert!(header_text.contains("name▲"), "header was {header_text}");
}

#[test]
fn sorting_job_ids_is_numeric() {
    let mut jobs = vec![test_job(0), test_job(1), test_job(2)];
    jobs[0].job_id = "9".to_string();
    jobs[0].array_id = "9".to_string();
    jobs[1].job_id = "100".to_string();
    jobs[1].array_id = "100".to_string();
    jobs[2].job_id = "99".to_string();
    jobs[2].array_id = "99".to_string();

    let mut app = app_with_jobs(jobs, Some(0));
    app.sort_jobs();

    assert_eq!(
        app.jobs.iter().map(Job::id).collect::<Vec<_>>(),
        vec!["100", "99", "9"]
    );
}

#[test]
fn sorting_preserves_selected_job_by_id() {
    let mut jobs = vec![test_job(0), test_job(1), test_job(2)];
    jobs[0].name = "alpha".to_string();
    jobs[1].name = "charlie".to_string();
    jobs[2].name = "bravo".to_string();

    let mut app = app_with_jobs(jobs, Some(2));
    let selected_before = app.selected_job_id();

    app.handle(AppMessage::Key(key('n')));

    assert_eq!(app.selected_job_id(), selected_before);
}

#[test]
fn ctrl_t_still_opens_time_limit_dialog() {
    let mut app = test_app(2, Some(0));

    app.handle(AppMessage::Key(key_with_modifiers(
        't',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert!(matches!(app.dialog, Some(Dialog::EditTimeLimit { .. })));
}

#[test]
fn selected_job_row_renders_below_header() {
    let mut app = test_app(4, Some(0));
    let buffer = draw_app(&mut app, 120, 12);
    let header_y = app.job_list_area.y.saturating_add(1);
    let first_row_y = app.job_list_area.y.saturating_add(2);
    let header_text = row_text(&buffer, app.job_list_area, header_y);
    let first_row_text = row_text(&buffer, app.job_list_area, first_row_y);

    assert!(header_text.contains("partition"));
    assert!(first_row_text.contains("job-0"));

    let job_name_x = app.job_list_area.x + first_row_text.find("job-0").unwrap() as u16;
    assert_eq!(
        buffer[(job_name_x, first_row_y)].style().bg,
        Some(Color::Green)
    );
}

#[test]
fn jobs_scrollbar_threshold_uses_only_body_rows() {
    let mut measuring_app = test_app(0, None);
    let _ = draw_app(&mut measuring_app, 120, 12);
    let visible_body_rows = usize::from(measuring_app.job_list_height);

    let mut app_without_overflow = test_app(visible_body_rows, Some(0));
    let buffer_without_overflow = draw_app(&mut app_without_overflow, 120, 12);
    let symbols_without_overflow = jobs_area_symbols(
        &buffer_without_overflow,
        app_without_overflow.job_list_rows_area(),
    );
    assert!(!symbols_without_overflow.iter().any(|symbol| symbol == "▲"));
    assert!(!symbols_without_overflow.iter().any(|symbol| symbol == "▼"));

    let mut app_with_overflow = test_app(visible_body_rows.saturating_add(1), Some(0));
    let buffer_with_overflow = draw_app(&mut app_with_overflow, 120, 12);
    let symbols_with_overflow = jobs_area_symbols(
        &buffer_with_overflow,
        app_with_overflow.job_list_rows_area(),
    );
    assert!(symbols_with_overflow.iter().any(|symbol| symbol == "▲"));
    assert!(symbols_with_overflow.iter().any(|symbol| symbol == "▼"));
    assert!(symbols_with_overflow.iter().any(|symbol| symbol == "█"));
}

#[test]
fn jobs_half_page_scroll_uses_body_row_height() {
    let mut app = test_app(30, Some(0));
    let _ = draw_app(&mut app, 120, 12);
    let body_rows = app.job_list_height;

    app.scroll_jobs_half_page_down();
    assert_eq!(
        app.job_list_state.selected(),
        Some((body_rows / 2) as usize)
    );

    app.scroll_jobs_half_page_up();
    assert_eq!(app.job_list_state.selected(), Some(0));
}

#[test]
fn jobs_scrollbar_thumb_tracks_the_table_offset() {
    let mut app_at_top = test_app(30, Some(0));
    let buffer_at_top = draw_app(&mut app_at_top, 120, 12);
    let thumb_top_at_top =
        scrollbar_thumb_top(&buffer_at_top, app_at_top.job_list_rows_area()).unwrap();

    let mut app_scrolled = test_app(30, Some(20));
    let buffer_scrolled = draw_app(&mut app_scrolled, 120, 12);
    let thumb_top_scrolled =
        scrollbar_thumb_top(&buffer_scrolled, app_scrolled.job_list_rows_area()).unwrap();

    assert!(app_scrolled.job_list_state.offset() > app_at_top.job_list_state.offset());
    assert!(thumb_top_scrolled > thumb_top_at_top);
}

#[test]
fn test_chunked_string() {
    let input = "abcdefghij";
    let expected = vec!["abcd", "ef", "gh", "ij"];
    assert_eq!(chunked_string(input, 4, 2), expected);

    let input = "123456789";
    let expected = vec!["1234", "56", "78", "9"];
    assert_eq!(chunked_string(input, 4, 2), expected);

    let input = "abc";
    let expected = vec!["abc"];
    assert_eq!(chunked_string(input, 4, 2), expected);

    let input = "abcde";
    let expected = vec!["abcd", "e"];
    assert_eq!(chunked_string(input, 4, 2), expected);

    let input = "";
    let expected: Vec<&str> = vec![""];
    assert_eq!(chunked_string(input, 4, 2), expected);

    let input = "123456789";
    let expected = vec!["1234", "56789"];
    assert_eq!(chunked_string(input, 4, 0), expected);

    let input = "123456789";
    let expected = vec!["12", "34", "56", "78", "9"];
    assert_eq!(chunked_string(input, 0, 2), expected);

    let input = "123456789";
    let expected = vec!["123456789"];
    assert_eq!(chunked_string(input, 0, 0), expected);
}

#[test]
fn test_validated_time_limit() {
    assert_eq!(validated_time_limit(&Input::new("".to_string())), None);
    assert_eq!(validated_time_limit(&Input::new("   ".to_string())), None);
    assert_eq!(
        validated_time_limit(&Input::new(" 01:00:00 ".to_string())),
        Some("01:00:00".to_string())
    );
}
