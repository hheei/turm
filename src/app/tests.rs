use super::commands::validated_time_limit;
use super::render::chunked_string;
use super::*;
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
        active_filter: String::new(),
        job_list_state: TableState::new().with_selected(selected),
        job_sort_field: JobSortField::Time,
        job_sort_direction: SortDirection::Asc,
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
        job_details_area: Rect::default(),
        job_output_area: Rect::default(),
        pending_input_event: None,
        pending_clipboard_copy: None,
        resource_table_state: TableState::new(),
        resource_list_height: 0,
        resource_area: Rect::default(),
        resources: Vec::new(),
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

fn visible_job_ids(app: &App) -> Vec<String> {
    app.visible_job_indices()
        .into_iter()
        .map(|index| app.jobs[index].id())
        .collect()
}

fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn buffer_text(buffer: &Buffer, width: u16, height: u16) -> String {
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn filter_test_jobs() -> Vec<Job> {
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

fn open_filter(app: &mut App) {
    app.handle(AppMessage::Key(key('f')));
}

fn type_in_filter(app: &mut App, value: &str) {
    for ch in value.chars() {
        app.handle(AppMessage::Key(key(ch)));
    }
}

fn key(char_key: char) -> KeyEvent {
    KeyEvent::new(
        KeyCode::Char(char_key),
        crossterm::event::KeyModifiers::NONE,
    )
}

fn special_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, crossterm::event::KeyModifiers::NONE)
}

fn key_with_modifiers(char_key: char, modifiers: crossterm::event::KeyModifiers) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(char_key), modifiers)
}

fn filtered_jobs_app() -> App {
    let mut app = app_with_jobs(filter_test_jobs(), Some(0));
    app.apply_job_filter("id:101512");
    app
}

fn copyable_jobs_app() -> App {
    let mut app = filtered_jobs_app();
    app.jobs[0].stdout = Some(PathBuf::from("/scratch/chlo/vasp-alpha/stdout.log"));
    app.jobs[0].stderr = Some(PathBuf::from("/scratch/chlo/vasp-alpha/stderr.log"));
    app
}

fn copyable_jobs_app_with_output_path(path: &str) -> App {
    let mut app = filtered_jobs_app();
    app.jobs[0].stdout = Some(PathBuf::from(path));
    app.jobs[0].stderr = Some(PathBuf::from(path));
    app
}

fn details_area(app: &App) -> Rect {
    app.job_details_area
}

fn border_fg(buffer: &Buffer, area: Rect) -> Option<Color> {
    buffer[(area.x, area.y)].style().fg
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
    assert!(header_text.contains("time▲"), "header was {header_text}");

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
    app.job_sort_field = JobSortField::Id;
    app.job_sort_direction = SortDirection::Desc;
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
    let buffer = draw_app(&mut app, 120, 14);
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
    let _ = draw_app(&mut measuring_app, 120, 20);
    let visible_body_rows = usize::from(measuring_app.job_list_height);

    let mut app_without_overflow = test_app(visible_body_rows, Some(0));
    let buffer_without_overflow = draw_app(&mut app_without_overflow, 120, 20);
    let symbols_without_overflow = jobs_area_symbols(
        &buffer_without_overflow,
        app_without_overflow.job_list_rows_area(),
    );
    assert!(!symbols_without_overflow.iter().any(|symbol| symbol == "▲"));
    assert!(!symbols_without_overflow.iter().any(|symbol| symbol == "▼"));

    let mut app_with_overflow = test_app(visible_body_rows.saturating_add(1), Some(0));
    let buffer_with_overflow = draw_app(&mut app_with_overflow, 120, 20);
    let symbols_with_overflow = jobs_area_symbols(
        &buffer_with_overflow,
        app_with_overflow.job_list_rows_area(),
    );
    assert!(symbols_with_overflow.iter().any(|symbol| symbol == "▲"));
    assert!(symbols_with_overflow.iter().any(|symbol| symbol == "▼"));
    assert!(symbols_with_overflow.iter().any(|symbol| symbol == "█"));
}

#[test]
fn jobs_half_page_up_uses_body_row_height() {
    let mut app = test_app(30, Some(20));
    let _ = draw_app(&mut app, 120, 20);
    let body_rows = app.job_list_height;
    let start = 20usize;

    app.scroll_jobs_half_page_up();
    assert_eq!(
        app.job_list_state.selected(),
        Some(start.saturating_sub((body_rows / 2) as usize))
    );
}

#[test]
fn jobs_scrollbar_thumb_tracks_the_table_offset() {
    let mut app_at_top = test_app(30, Some(0));
    let buffer_at_top = draw_app(&mut app_at_top, 120, 20);
    let thumb_top_at_top =
        scrollbar_thumb_top(&buffer_at_top, app_at_top.job_list_rows_area()).unwrap();

    let mut app_scrolled = test_app(30, Some(20));
    let buffer_scrolled = draw_app(&mut app_scrolled, 120, 20);
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

#[test]
fn f_opens_filter_dialog_with_active_filter_draft() {
    let mut app = test_app(2, Some(0));
    app.active_filter = "name:vasp".to_string();

    open_filter(&mut app);

    match app.dialog.as_ref() {
        Some(Dialog::FilterJobs { input }) => assert_eq!(input.value(), "name:vasp"),
        _ => panic!("expected filter dialog"),
    }
}

#[test]
fn filter_dialog_accepts_q_without_quitting_or_sorting() {
    let mut app = test_app(3, Some(0));
    open_filter(&mut app);

    let (should_quit, should_draw) = app.handle_input_event(Event::Key(key('q')));
    assert!(!should_quit);
    assert!(should_draw);
    assert_eq!(app.job_sort_field, JobSortField::Time);

    match app.dialog.as_ref() {
        Some(Dialog::FilterJobs { input }) => assert_eq!(input.value(), "q"),
        _ => panic!("expected filter dialog"),
    }
}

#[test]
fn typing_enter_esc_and_clear_work_in_filter_dialog() {
    let mut app = app_with_jobs(filter_test_jobs(), Some(0));

    open_filter(&mut app);
    app.handle(AppMessage::Key(key('n')));
    assert_eq!(app.job_sort_field, JobSortField::Time);

    type_in_filter(&mut app, "ame:vasp");
    app.handle(AppMessage::Key(key_with_modifiers(
        'u',
        crossterm::event::KeyModifiers::CONTROL,
    )));
    match app.dialog.as_ref() {
        Some(Dialog::FilterJobs { input }) => assert_eq!(input.value(), ""),
        _ => panic!("expected filter dialog"),
    }

    type_in_filter(&mut app, "name:vasp");
    app.handle(AppMessage::Key(KeyEvent::new(
        KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    )));
    assert!(app.dialog.is_none());
    assert_eq!(app.active_filter, "name:vasp");
    assert_eq!(visible_job_ids(&app), ids(&["101512"]));

    open_filter(&mut app);
    type_in_filter(&mut app, "-edited");
    app.handle(AppMessage::Key(KeyEvent::new(
        KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    )));
    assert!(app.dialog.is_none());
    assert_eq!(app.active_filter, "name:vasp");
}

#[test]
fn free_text_filter_matches_visible_job_fields() {
    let expectations: [(&str, &[&str]); 6] = [
        ("vasp", &["101512"]),
        ("gpu", &["101512"]),
        ("chlo", &["101512", "303333"]),
        ("PD", &["202000"]),
        ("7:00", &["101512"]),
        ("101512", &["101512"]),
    ];

    for (query, expected) in expectations {
        let mut app = app_with_jobs(filter_test_jobs(), Some(0));
        app.apply_job_filter(query);
        assert_eq!(visible_job_ids(&app), ids(expected), "query {query}");
    }
}

#[test]
fn field_filters_match_expected_columns() {
    let expectations: [(&str, &[&str]); 10] = [
        ("job:101512", &["101512"]),
        ("job:vasp", &["101512"]),
        ("id:202000", &["202000"]),
        ("name:analysis", &["303333"]),
        ("user:alex", &["202000"]),
        ("partition:gpu", &["101512"]),
        ("part:cpu", &["303333"]),
        ("state:running", &["101512"]),
        ("st:PD", &["202000"]),
        ("time:01:23", &["303333"]),
    ];

    for (query, expected) in expectations {
        let mut app = app_with_jobs(filter_test_jobs(), Some(0));
        app.apply_job_filter(query);
        assert_eq!(visible_job_ids(&app), ids(expected), "query {query}");
    }
}

#[test]
fn unknown_field_prefix_falls_back_to_free_text() {
    let mut jobs = filter_test_jobs();
    jobs[2].name = "unknown:needle".to_string();
    let mut app = app_with_jobs(jobs, Some(0));

    app.apply_job_filter("unknown:needle");

    assert_eq!(visible_job_ids(&app), ids(&["303333"]));
}

#[test]
fn filtering_preserves_selection_and_uses_visible_fallbacks() {
    let mut app = app_with_jobs(filter_test_jobs(), Some(1));

    app.apply_job_filter("state:pd");
    assert_eq!(app.selected_job_id(), Some("202000".to_string()));
    assert_eq!(app.job_list_state.selected(), Some(0));

    app.apply_job_filter("user:chlo");
    assert_eq!(app.selected_job_id(), Some("101512".to_string()));
    assert_eq!(app.job_list_state.selected(), Some(0));

    app.apply_job_filter("name:missing");
    assert_eq!(app.selected_job_id(), None);
    assert_eq!(app.job_list_state.selected(), None);

    app.apply_job_filter("");
    assert_eq!(visible_job_ids(&app), ids(&["101512", "202000", "303333"]));
    assert_eq!(app.selected_job_id(), Some("101512".to_string()));
    assert_eq!(app.job_list_state.selected(), Some(0));
}

#[test]
fn filtering_keeps_active_sort_order() {
    let mut app = app_with_jobs(filter_test_jobs(), Some(0));

    app.handle(AppMessage::Key(key('n')));
    app.handle(AppMessage::Key(key('n')));
    app.apply_job_filter("user:chlo");

    assert_eq!(app.job_sort_field, JobSortField::Name);
    assert_eq!(app.job_sort_direction, SortDirection::Asc);
    assert_eq!(visible_job_ids(&app), ids(&["303333", "101512"]));
}

#[test]
fn filtered_jobs_title_scrollbar_and_empty_state_use_visible_count() {
    let mut app = app_with_jobs(filter_test_jobs(), Some(0));
    app.apply_job_filter("name:missing");

    let buffer = draw_app(&mut app, 120, 12);
    let header_y = app.job_list_area.y.saturating_add(1);
    let header_text = row_text(&buffer, app.job_list_area, header_y);
    let all_text = buffer_text(&buffer, 120, 12);
    let symbols = jobs_area_symbols(&buffer, app.job_list_rows_area());

    assert!(all_text.contains("Jobs (0/3) filter: name:missing"));
    assert!(header_text.contains("partition"));
    assert!(!all_text.contains("vasp-alpha"));
    assert!(!symbols.iter().any(|symbol| symbol == "▲"));
    assert!(!symbols.iter().any(|symbol| symbol == "▼"));
    assert_eq!(app.selected_job_id(), None);
}

#[test]
fn filter_popup_renders_over_base_ui_with_title() {
    let mut app = app_with_jobs(filter_test_jobs(), Some(0));
    open_filter(&mut app);
    type_in_filter(&mut app, "gpu");

    let buffer = draw_app(&mut app, 120, 12);
    let all_text = buffer_text(&buffer, 120, 12);
    let header_y = app.job_list_area.y.saturating_add(1);
    let header_text = row_text(&buffer, app.job_list_area, header_y);

    assert!(all_text.contains("Filter:"));
    assert!(all_text.contains("gpu"));
    assert!(header_text.contains("partition"));
}

#[test]
fn bracket_keys_cycle_focus_forward_and_backward() {
    let mut app = test_app(3, Some(0));

    assert_eq!(app.focus, Focus::Jobs);

    app.handle(AppMessage::Key(key(']')));
    assert_eq!(app.focus, Focus::Details);

    app.handle(AppMessage::Key(key(']')));
    assert_eq!(app.focus, Focus::Log);

    app.handle(AppMessage::Key(key(']')));
    assert_eq!(app.focus, Focus::Resources);

    app.handle(AppMessage::Key(key('[')));
    assert_eq!(app.focus, Focus::Log);

    app.handle(AppMessage::Key(key('[')));
    assert_eq!(app.focus, Focus::Details);

    app.handle(AppMessage::Key(key('[')));
    assert_eq!(app.focus, Focus::Jobs);
}

#[test]
fn tab_h_l_and_arrow_keys_do_not_switch_focus() {
    let mut app = test_app(3, Some(0));
    app.focus = Focus::Details;

    for key_event in [
        special_key(KeyCode::Tab),
        special_key(KeyCode::BackTab),
        key('h'),
        key('l'),
        special_key(KeyCode::Left),
        special_key(KeyCode::Right),
    ] {
        app.handle(AppMessage::Key(key_event));
        assert_eq!(app.focus, Focus::Details);
    }
}

#[test]
fn filter_dialog_keeps_bracket_input_and_focus() {
    let mut app = test_app(3, Some(0));
    open_filter(&mut app);

    app.handle(AppMessage::Key(key(']')));
    app.handle(AppMessage::Key(key('[')));

    assert_eq!(app.focus, Focus::Jobs);
    match app.dialog.as_ref() {
        Some(Dialog::FilterJobs { input }) => assert_eq!(input.value(), "]["),
        _ => panic!("expected filter dialog"),
    }
}

#[test]
fn jobs_navigation_stays_in_jobs_focus() {
    let mut app = test_app(4, Some(0));

    app.handle(AppMessage::Key(key('j')));
    assert_eq!(app.job_list_state.selected(), Some(1));

    app.handle(AppMessage::Key(key('k')));
    assert_eq!(app.job_list_state.selected(), Some(0));
}

#[test]
fn log_focus_routes_navigation_to_log_scrolling() {
    let mut app = test_app(30, Some(2));
    let _ = draw_app(&mut app, 120, 16);
    app.focus = Focus::Log;
    app.job_output_anchor = ScrollAnchor::Top;
    app.job_output_offset = 0;

    app.handle(AppMessage::Key(key('j')));
    assert_eq!(app.job_list_state.selected(), Some(2));
    assert_eq!(app.job_output_offset, 1);

    app.handle(AppMessage::Key(special_key(KeyCode::PageDown)));
    assert!(app.job_output_offset > 1);

    app.handle(AppMessage::Key(key('G')));
    assert_eq!(app.job_output_anchor, ScrollAnchor::Bottom);
    assert_eq!(app.job_output_offset, 0);

    app.handle(AppMessage::Key(key('g')));
    assert_eq!(app.job_output_anchor, ScrollAnchor::Top);
    assert_eq!(app.job_output_offset, 0);

    app.handle(AppMessage::Key(key('w')));
    assert!(app.job_output_wrap);
}

#[test]
fn focused_border_style_tracks_the_active_panel() {
    let mut jobs_app = test_app(3, Some(0));
    let jobs_buffer = draw_app(&mut jobs_app, 120, 20);
    assert_eq!(
        border_fg(&jobs_buffer, jobs_app.job_list_area),
        Some(Color::Green)
    );
    assert_ne!(
        border_fg(&jobs_buffer, details_area(&jobs_app)),
        Some(Color::Green)
    );
    assert_ne!(
        border_fg(&jobs_buffer, jobs_app.job_output_area),
        Some(Color::Green)
    );

    let mut details_app = test_app(3, Some(0));
    details_app.focus = Focus::Details;
    let details_buffer = draw_app(&mut details_app, 120, 20);
    assert_ne!(
        border_fg(&details_buffer, details_app.job_list_area),
        Some(Color::Green)
    );
    assert_eq!(
        border_fg(&details_buffer, details_area(&details_app)),
        Some(Color::Green)
    );
    assert_ne!(
        border_fg(&details_buffer, details_app.job_output_area),
        Some(Color::Green)
    );

    let mut log_app = test_app(3, Some(0));
    log_app.focus = Focus::Log;
    let log_buffer = draw_app(&mut log_app, 120, 20);
    assert_ne!(
        border_fg(&log_buffer, log_app.job_list_area),
        Some(Color::Green)
    );
    assert_ne!(
        border_fg(&log_buffer, details_area(&log_app)),
        Some(Color::Green)
    );
    assert_eq!(
        border_fg(&log_buffer, log_app.job_output_area),
        Some(Color::Green)
    );
}

#[test]
fn help_line_reflects_the_active_focus() {
    let mut jobs_app = test_app(3, Some(0));
    let jobs_text = buffer_text(&draw_app(&mut jobs_app, 120, 12), 120, 12);
    assert!(!jobs_text.contains("mode: jobs"));
    assert!(jobs_text.contains("c: copy"));
    assert!(jobs_text.contains("^d: cancel"));
    assert!(!jobs_text.contains("s/p/i/n/u/t: sort"));
    assert!(!jobs_text.contains("j/k: move"));

    jobs_app = copyable_jobs_app();
    jobs_app.handle(AppMessage::Key(key('c')));
    let copy_text = buffer_text(&draw_app(&mut jobs_app, 120, 20), 120, 20);
    assert!(copy_text.contains("mode: copy"));
    assert!(copy_text.contains("c: dir-url"));
    assert!(copy_text.contains("d: dir-name"));
    let mut details_app = test_app(3, Some(0));
    details_app.focus = Focus::Details;
    let details_text = buffer_text(&draw_app(&mut details_app, 120, 12), 120, 12);
    assert!(details_text.contains("mode: details"));
    assert!(details_text.contains("^d: cancel"));

    let mut log_app = test_app(3, Some(0));
    log_app.focus = Focus::Log;
    let log_text = buffer_text(&draw_app(&mut log_app, 120, 12), 120, 12);
    assert!(log_text.contains("mode: log"));
    assert!(log_text.contains("^d: cancel"));
    assert!(log_text.contains("w: wrap"));
}

#[test]
fn mouse_click_focuses_details_and_log_panels() {
    let mut app = test_app(4, Some(0));
    let _ = draw_app(&mut app, 120, 20);

    let details = details_area(&app);
    app.handle(AppMessage::MouseClick {
        column: details.x.saturating_add(1),
        row: details.y.saturating_add(1),
    });
    assert_eq!(app.focus, Focus::Details);

    let log = app.job_output_area;
    app.handle(AppMessage::MouseClick {
        column: log.x.saturating_add(1),
        row: log.y.saturating_add(1),
    });
    assert_eq!(app.focus, Focus::Log);
}

#[test]
fn mouse_click_on_jobs_restores_jobs_focus_and_selection() {
    let mut app = test_app(6, Some(0));
    let buffer = draw_app(&mut app, 120, 20);
    app.focus = Focus::Log;

    let first_row_y = app.job_list_area.y.saturating_add(2);
    let first_row_text = row_text(&buffer, app.job_list_area, first_row_y);
    let job_name_x = app.job_list_area.x + first_row_text.find("job-0").unwrap() as u16;

    app.handle(AppMessage::MouseClick {
        column: job_name_x,
        row: first_row_y,
    });

    assert_eq!(app.focus, Focus::Jobs);
    assert_eq!(app.job_list_state.selected(), Some(0));
}

#[test]
fn dialog_blocks_mouse_focus_changes() {
    let mut app = test_app(4, Some(0));
    let _ = draw_app(&mut app, 120, 20);
    app.dialog = Some(Dialog::FilterJobs {
        input: Input::new(String::new()),
    });

    let details = details_area(&app);
    let (should_quit, should_draw) =
        app.handle_input_event(Event::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: details.x.saturating_add(1),
            row: details.y.saturating_add(1),
            modifiers: crossterm::event::KeyModifiers::NONE,
        }));

    assert!(!should_quit);
    assert!(!should_draw);
    assert_eq!(app.focus, Focus::Jobs);
}

#[test]
fn ctrl_d_in_jobs_focus_opens_cancel_confirmation_popup() {
    let mut app = filtered_jobs_app();

    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    match app.dialog.as_ref() {
        Some(Dialog::ConfirmCancelJob {
            id, name, signal, ..
        }) => {
            assert_eq!(id, "101512");
            assert_eq!(name, "vasp-alpha");
            assert_eq!(*signal, None);
        }
        _ => panic!("expected confirm cancel dialog"),
    }
}

#[test]
fn ctrl_d_in_details_focus_opens_cancel_popup() {
    let mut app = filtered_jobs_app();
    app.focus = Focus::Details;

    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert!(matches!(app.dialog, Some(Dialog::ConfirmCancelJob { .. })));
}

#[test]
fn ctrl_d_in_log_focus_opens_cancel_popup() {
    let mut app = filtered_jobs_app();
    app.focus = Focus::Log;
    app.job_output_anchor = ScrollAnchor::Top;
    app.job_output_offset = 3;

    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert_eq!(app.job_output_offset, 3);
    assert!(matches!(app.dialog, Some(Dialog::ConfirmCancelJob { .. })));
}

#[test]
fn c_opens_copy_popup_and_uppercase_c_stays_unbound() {
    let mut app = copyable_jobs_app();

    app.handle(AppMessage::Key(key('c')));
    assert!(matches!(
        app.dialog,
        Some(Dialog::CopyJobOutputDirectory { .. })
    ));

    app.dialog = None;
    app.handle(AppMessage::Key(key('C')));
    assert!(app.dialog.is_none());
}

#[test]
fn copy_popup_renders_directory_actions() {
    let mut app = copyable_jobs_app();
    app.handle(AppMessage::Key(key('c')));

    let text = buffer_text(&draw_app(&mut app, 120, 20), 120, 20);
    assert!(text.contains("Copy:"));
    assert!(text.contains("copy dir url"));
    assert!(text.contains("/scratch/chlo/vasp-alpha"));
    assert!(text.contains("copy directory name"));
    assert!(text.contains("vasp-alpha"));
}

#[test]
fn copy_popup_copies_directory_url_with_c() {
    let mut app = copyable_jobs_app();
    app.handle(AppMessage::Key(key('c')));
    app.handle(AppMessage::Key(key('c')));

    assert!(app.dialog.is_none());
    assert_eq!(
        app.pending_clipboard_copy.as_deref(),
        Some("/scratch/chlo/vasp-alpha")
    );
}

#[test]
fn copy_popup_preserves_special_characters_in_directory_path() {
    let mut app = copyable_jobs_app_with_output_path(
        "/scratch/chlo/vasp+alpha %beta#gamma?delta&epsilon/stdout.log",
    );
    app.handle(AppMessage::Key(key('c')));
    app.handle(AppMessage::Key(key('c')));

    assert!(app.dialog.is_none());
    assert_eq!(
        app.pending_clipboard_copy.as_deref(),
        Some("/scratch/chlo/vasp+alpha %beta#gamma?delta&epsilon")
    );
}

#[test]
fn copy_popup_copies_directory_name_with_d() {
    let mut app = copyable_jobs_app();
    app.handle(AppMessage::Key(key('c')));
    app.handle(AppMessage::Key(key('d')));

    assert!(app.dialog.is_none());
    assert_eq!(app.pending_clipboard_copy.as_deref(), Some("vasp-alpha"));
}

#[test]
fn copy_popup_ignores_plain_d_until_opened() {
    let mut app = copyable_jobs_app();
    app.handle(AppMessage::Key(key('d')));

    assert!(app.dialog.is_none());
    assert!(app.pending_clipboard_copy.is_none());
}

#[test]
fn cancel_confirmation_popup_renders_job_details_and_buttons() {
    let mut app = filtered_jobs_app();
    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    let text = buffer_text(&draw_app(&mut app, 120, 20), 120, 20);
    assert!(text.contains("Cancel selected job?"));
    assert!(text.contains("Job 101512"));
    assert!(text.contains("vasp-alpha"));
    assert!(text.contains("[Y]es"));
    assert!(text.contains("(N)o"));
}

#[test]
fn cancel_confirmation_accepts_safe_close_keys() {
    for key_event in [key('n'), key('N'), special_key(KeyCode::Esc)] {
        let mut app = filtered_jobs_app();
        app.handle(AppMessage::Key(key_with_modifiers(
            'd',
            crossterm::event::KeyModifiers::CONTROL,
        )));

        app.handle(AppMessage::Key(key_event));
        assert!(app.dialog.is_none());
    }
}

#[test]
fn cancel_confirmation_maps_yes_keys_without_running_scancel() {
    use super::events::{CancelConfirmationAction, cancel_confirmation_action};

    for key_event in [key('y'), key('Y'), special_key(KeyCode::Enter)] {
        assert_eq!(
            cancel_confirmation_action(key_event),
            CancelConfirmationAction::Confirm
        );
    }
}

#[test]
fn ctrl_d_with_no_selected_job_does_not_panic_or_open_dialog() {
    let mut app = test_app(0, None);

    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert!(app.dialog.is_none());
}

#[test]
fn ctrl_d_in_filter_dialog_does_not_open_cancel_popup() {
    let mut app = filtered_jobs_app();
    open_filter(&mut app);

    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert!(matches!(app.dialog, Some(Dialog::FilterJobs { .. })));
}

// ── Resources panel tests ──

#[test]
fn resources_panel_renders_title_and_headers() {
    let mut app = test_app(3, Some(0));
    let buffer = draw_app(&mut app, 120, 20);
    let text = buffer_text(&buffer, 120, 20);
    assert!(text.contains("Resources (nodes)"));
    assert!(text.contains("Partition"));
    assert!(text.contains("Running"));
    assert!(text.contains("Available"));
    assert!(!text.contains("Pending"));
}

#[test]
fn resources_panel_shows_empty_state_when_no_data() {
    let mut app = test_app(3, Some(0));
    let buffer = draw_app(&mut app, 120, 20);
    let text = buffer_text(&buffer, 120, 20);
    assert!(text.contains("No resource"), "text was:\n{text}");
}

#[test]
fn focus_cycles_include_resources_panel() {
    let mut app = test_app(3, Some(0));

    // Default is Jobs
    assert_eq!(app.focus, Focus::Jobs);

    // Jobs -> Resources (via [)
    app.handle(AppMessage::Key(key('[')));
    assert_eq!(app.focus, Focus::Resources);

    // Resources -> Log (via [)
    app.handle(AppMessage::Key(key('[')));
    assert_eq!(app.focus, Focus::Log);

    // Log -> Details (via [)
    app.handle(AppMessage::Key(key('[')));
    assert_eq!(app.focus, Focus::Details);

    // Details -> Jobs (via [)
    app.handle(AppMessage::Key(key('[')));
    assert_eq!(app.focus, Focus::Jobs);

    // Jobs -> Details (via ])
    app.handle(AppMessage::Key(key(']')));
    assert_eq!(app.focus, Focus::Details);

    // Details -> Log (via ])
    app.handle(AppMessage::Key(key(']')));
    assert_eq!(app.focus, Focus::Log);

    // Log -> Resources (via ])
    app.handle(AppMessage::Key(key(']')));
    assert_eq!(app.focus, Focus::Resources);

    // Resources -> Jobs (via ])
    app.handle(AppMessage::Key(key(']')));
    assert_eq!(app.focus, Focus::Jobs);
}

#[test]
fn resources_scrollbar_hidden_when_rows_fit() {
    let mut app = test_app(3, Some(0));
    let buffer = draw_app(&mut app, 120, 20);
    let symbols = jobs_area_symbols(&buffer, app.resource_area);
    // With empty resources (1 placeholder row), row fits in 8-height panel
    assert!(!symbols.iter().any(|s| s == "▲"));
    assert!(!symbols.iter().any(|s| s == "▼"));
}

#[test]
fn help_line_shows_resources_mode_when_focused() {
    let mut app = test_app(3, Some(0));
    app.focus = Focus::Resources;
    let text = buffer_text(&draw_app(&mut app, 120, 20), 120, 20);
    assert!(text.contains("mode: resources"));
    assert!(text.contains("j/k"));
    assert!(text.contains("g/G"));
}

#[test]
fn resources_panel_uses_top_left_area() {
    let mut app = test_app(3, Some(0));
    let _ = draw_app(&mut app, 100, 20);
    // Resources is top-left, same width as Jobs
    assert_eq!(app.resource_area.x, 0);
    assert_eq!(app.resource_area.y, 0);
    assert_eq!(app.resource_area.width, app.job_list_area.width);
    // Resources height equals Details height
    assert_eq!(app.resource_area.height, app.job_details_area.height);
}
