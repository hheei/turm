mod common;
use common::*;

#[test]
fn sort_keys_toggle_direction_and_switch_fields() {
    let mut app = test_app(3, Some(0));

    app.handle(AppMessage::Key(key('n')));
    assert_eq!(app.sort_field(), JobSortField::Name);
    assert_eq!(app.sort_direction(), SortDirection::Desc);

    app.handle(AppMessage::Key(key('n')));
    assert_eq!(app.sort_field(), JobSortField::Name);
    assert_eq!(app.sort_direction(), SortDirection::Asc);

    app.handle(AppMessage::Key(key('p')));
    assert_eq!(app.sort_field(), JobSortField::Partition);
    assert_eq!(app.sort_direction(), SortDirection::Desc);

    app.handle(AppMessage::Key(key('j')));
    assert_eq!(app.sort_field(), JobSortField::Id);
    assert_eq!(app.sort_direction(), SortDirection::Desc);
}

#[test]
fn header_renders_active_sort_indicator() {
    let mut app = test_app(2, Some(0));
    let buffer = draw_app(&mut app, 120, 12);
    let header_y = app.job_list_area().y.saturating_add(1);
    let header_text = row_text(&buffer, app.job_list_area(), header_y);
    assert!(header_text.contains("time▲"), "header was {header_text}");

    app.handle(AppMessage::Key(key('n')));
    app.handle(AppMessage::Key(key('n')));
    let buffer = draw_app(&mut app, 120, 12);
    let header_text = row_text(&buffer, app.job_list_area(), header_y);
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
    app.set_sort(JobSortField::Id, SortDirection::Desc);
    app.sort_jobs();

    assert_eq!(app.job_ids(), vec!["100", "99", "9"]);
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
fn default_time_sort_places_completed_jobs_last() {
    let mut jobs = vec![test_job(0), test_job(1), test_job(2)];
    jobs[0].time = "00:01:00".to_string();
    jobs[1].time = "00:03:00".to_string();
    jobs[1].state_compact = "CD".to_string();
    jobs[2].time = "00:02:00".to_string();

    let mut app = app_with_jobs(jobs, Some(0));
    app.set_sort(JobSortField::Time, SortDirection::Asc);
    app.sort_jobs();

    assert_eq!(app.job_ids(), vec!["1000", "1002", "1001"]);
}

#[test]
fn completed_jobs_stay_last_for_explicit_sorting() {
    let mut jobs = vec![test_job(0), test_job(1), test_job(2)];
    jobs[0].name = "zeta".to_string();
    jobs[1].name = "alpha".to_string();
    jobs[1].state_compact = "CD".to_string();
    jobs[2].name = "beta".to_string();

    let mut app = app_with_jobs(jobs, Some(0));
    app.set_sort(JobSortField::Name, SortDirection::Asc);
    app.sort_jobs();

    assert_eq!(app.job_ids(), vec!["1002", "1000", "1001"]);
}

#[test]
fn f_opens_filter_dialog_with_active_filter_draft() {
    let mut app = test_app(2, Some(0));
    app.set_active_filter("name:vasp".to_string());

    open_filter(&mut app);

    match app.dialog() {
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
    assert_eq!(app.sort_field(), JobSortField::Time);

    match app.dialog() {
        Some(Dialog::FilterJobs { input }) => assert_eq!(input.value(), "q"),
        _ => panic!("expected filter dialog"),
    }
}

#[test]
fn typing_enter_esc_and_clear_work_in_filter_dialog() {
    let mut app = app_with_jobs(filter_test_jobs(), Some(0));

    open_filter(&mut app);
    app.handle(AppMessage::Key(key('n')));
    assert_eq!(app.sort_field(), JobSortField::Time);

    type_in_filter(&mut app, "ame:vasp");
    app.handle(AppMessage::Key(key_with_modifiers(
        'u',
        crossterm::event::KeyModifiers::CONTROL,
    )));
    match app.dialog() {
        Some(Dialog::FilterJobs { input }) => assert_eq!(input.value(), ""),
        _ => panic!("expected filter dialog"),
    }

    type_in_filter(&mut app, "name:vasp");
    app.handle(AppMessage::Key(KeyEvent::new(
        KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    )));
    assert!(app.dialog().is_none());
    assert_eq!(app.active_filter(), "name:vasp");
    assert_eq!(visible_job_ids(&app), ids(&["101512"]));

    open_filter(&mut app);
    type_in_filter(&mut app, "-edited");
    app.handle(AppMessage::Key(KeyEvent::new(
        KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    )));
    assert!(app.dialog().is_none());
    assert_eq!(app.active_filter(), "name:vasp");
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
    assert_eq!(app.selected_job_index(), Some(0));

    app.apply_job_filter("user:chlo");
    assert_eq!(app.selected_job_id(), Some("101512".to_string()));
    assert_eq!(app.selected_job_index(), Some(0));

    app.apply_job_filter("name:missing");
    assert_eq!(app.selected_job_id(), None);
    assert_eq!(app.selected_job_index(), None);

    app.apply_job_filter("");
    assert_eq!(visible_job_ids(&app), ids(&["101512", "202000", "303333"]));
    assert_eq!(app.selected_job_id(), Some("101512".to_string()));
    assert_eq!(app.selected_job_index(), Some(0));
}

#[test]
fn filtering_keeps_active_sort_order() {
    let mut app = app_with_jobs(filter_test_jobs(), Some(0));

    app.handle(AppMessage::Key(key('n')));
    app.handle(AppMessage::Key(key('n')));
    app.apply_job_filter("user:chlo");

    assert_eq!(app.sort_field(), JobSortField::Name);
    assert_eq!(app.sort_direction(), SortDirection::Asc);
    assert_eq!(visible_job_ids(&app), ids(&["303333", "101512"]));
}

#[test]
fn filtered_jobs_title_scrollbar_and_empty_state_use_visible_count() {
    let mut app = app_with_jobs(filter_test_jobs(), Some(0));
    app.apply_job_filter("name:missing");

    let buffer = draw_app(&mut app, 120, 12);
    let header_y = app.job_list_area().y.saturating_add(1);
    let header_text = row_text(&buffer, app.job_list_area(), header_y);
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

    assert!(all_text.contains("Filter:"));
    assert!(all_text.contains("gpu"));
    assert!(all_text.contains("Res"));
}
