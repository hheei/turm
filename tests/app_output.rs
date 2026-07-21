mod common;
use common::*;

#[test]
fn collapsing_output_moves_log_focus_to_jobs() {
    let mut app = test_app(1, Some(0));
    app.set_focus(Focus::Log);
    app.set_output_mode(OutputPanelMode::Stderr);
    let _ = draw_app(&mut app, 120, 40);

    app.handle(AppMessage::Key(key('o')));

    assert_eq!(app.output_mode(), OutputPanelMode::Collapsed);
    assert_eq!(app.focus(), Focus::Jobs);
}

#[test]
fn leaving_collapsed_keeps_jobs_focus() {
    let mut app = test_app(1, Some(0));
    app.set_focus(Focus::Jobs);
    app.set_output_mode(OutputPanelMode::Collapsed);
    let _ = draw_app(&mut app, 120, 40);

    app.handle(AppMessage::Key(key('o')));

    assert_eq!(app.output_mode(), OutputPanelMode::Workdir);
    assert_eq!(app.focus(), Focus::Jobs);
}

#[test]
fn output_mode_switch_resets_tail_position() {
    let mut app = test_app(1, Some(0));
    app.set_focus(Focus::Log);
    app.set_output_mode(OutputPanelMode::Stdout);
    app.set_output_anchor(ScrollAnchor::Top);
    app.set_output_offset(7);
    let _ = draw_app(&mut app, 120, 40);

    app.handle(AppMessage::Key(key('o')));

    assert_eq!(app.output_mode(), OutputPanelMode::Stderr);
    assert_eq!(app.output_anchor(), ScrollAnchor::Bottom);
    assert_eq!(app.output_offset(), 0);
}

#[test]
fn horizontal_output_scroll_clamps_and_wrap_resets_offset() {
    let mut app =
        app_with_output_lines(&["012345678901234567890123456789012345678901234567890123456789"]);
    let _ = draw_app(&mut app, 24, 29);
    app.set_focus(Focus::Log);
    app.set_output_wrap(false);

    app.handle(AppMessage::Key(key('L')));
    assert_eq!(app.output_scroll_x(), app.layout().viewport.width / 2);

    for _ in 0..u16::MAX {
        app.handle(AppMessage::Key(key('L')));
    }
    let max_x = app.max_output_scroll_x();
    assert!(max_x > 0);
    assert_eq!(app.output_scroll_x(), max_x);

    app.handle(AppMessage::Key(key('H')));
    assert!(app.output_scroll_x() < max_x);
    for _ in 0..u16::MAX {
        app.handle(AppMessage::Key(key('H')));
    }
    assert_eq!(app.output_scroll_x(), 0);

    app.handle(AppMessage::Key(key('w')));
    assert!(app.output_wrap());
    assert_eq!(app.output_scroll_x(), 0);
}

#[test]
fn output_rendering_clips_without_ellipsis() {
    let mut app = app_with_output_lines(&["abcdefghijABCDEFGHIJ0123456789"]);
    app.set_focus(Focus::Log);
    app.set_output_wrap(false);

    let text = buffer_text(&draw_app(&mut app, 30, 29), 30, 29);

    assert!(!text.contains("..."));
    assert!(!text.contains("…"));
}

#[test]
fn output_scrollbars_follow_wrap_and_overflow() {
    let mut app = app_with_output_lines(&[
        "0123456789012345678901234567890123456789",
        "line 2",
        "line 3",
        "line 4",
        "line 5",
        "line 6",
        "line 7",
        "line 8",
        "line 9",
        "line 10",
        "line 11",
        "line 12",
    ]);
    app.set_focus(Focus::Log);
    app.set_output_wrap(false);

    let buffer = draw_app(&mut app, 40, 29);
    let symbols = jobs_area_symbols(&buffer, app.job_output_area());
    assert!(symbols.iter().any(|s| s == VERTICAL_SCROLLBAR_THUMB));
    assert!(
        symbols
            .iter()
            .any(|s| s == OUTPUT_HORIZONTAL_SCROLLBAR_THUMB)
    );
    assert!(
        !symbols
            .iter()
            .any(|s| matches!(s.as_str(), "▲" | "▼" | "◄" | "►"))
    );

    let horizontal_area = output_horizontal_scrollbar_area(&app).unwrap();
    assert!(
        !symbol_columns(&buffer, horizontal_area, OUTPUT_HORIZONTAL_SCROLLBAR_THUMB).is_empty()
    );

    app.handle(AppMessage::Key(key('w')));
    let wrapped = draw_app(&mut app, 40, 29);
    assert!(output_horizontal_scrollbar_area(&app).is_none());
    assert!(
        symbol_columns(&wrapped, horizontal_area, OUTPUT_HORIZONTAL_SCROLLBAR_THUMB).is_empty()
    );
}

#[test]
fn output_vertical_scrollbar_matches_top_and_bottom_positions() {
    let mut app = app_with_output_lines(
        &(0..24)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    app.set_focus(Focus::Log);
    app.set_output_wrap(false);

    let top_buffer = draw_app(&mut app, 40, 29);
    let vertical_area = output_vertical_scrollbar_area(&app).unwrap();
    let thumb_rows_at_bottom = symbol_rows(&top_buffer, vertical_area, VERTICAL_SCROLLBAR_THUMB);

    app.handle(AppMessage::Key(key('g')));
    let topmost_buffer = draw_app(&mut app, 40, 29);
    let thumb_rows_at_top = symbol_rows(&topmost_buffer, vertical_area, VERTICAL_SCROLLBAR_THUMB);

    assert_eq!(app.output_anchor(), ScrollAnchor::Top);
    assert_eq!(app.output_offset(), 0);
    assert_eq!(thumb_rows_at_top.first().copied(), Some(vertical_area.y));
    assert!(
        thumb_rows_at_top.last().copied().unwrap() < vertical_area.y + vertical_area.height - 1
    );

    app.handle(AppMessage::Key(key('G')));
    let bottom_buffer = draw_app(&mut app, 40, 29);
    let thumb_rows_back_at_bottom =
        symbol_rows(&bottom_buffer, vertical_area, VERTICAL_SCROLLBAR_THUMB);

    assert_eq!(app.output_anchor(), ScrollAnchor::Bottom);
    assert_eq!(app.output_offset(), 0);
    assert_eq!(
        thumb_rows_back_at_bottom.last().copied(),
        Some(vertical_area.y + vertical_area.height - 1)
    );
    assert_eq!(thumb_rows_back_at_bottom, thumb_rows_at_bottom);
}

#[test]
fn changing_selected_job_resets_tail_position() {
    let mut app = test_app(4, Some(0));
    app.set_output_mode(OutputPanelMode::Stdout);
    app.set_output_anchor(ScrollAnchor::Top);
    app.set_output_offset(5);

    app.handle(AppMessage::Key(special_key(KeyCode::Down)));

    assert_eq!(app.selected_job_index(), Some(1));
    assert_eq!(app.output_anchor(), ScrollAnchor::Bottom);
    assert_eq!(app.output_offset(), 0);
}

#[test]
fn watched_output_path_matches_selected_mode() {
    let mut job = test_job(0);
    job.stdout = Some(PathBuf::from("/tmp/stdout.log"));
    job.stderr = Some(PathBuf::from("/tmp/stderr.log"));

    assert_eq!(
        watched_output_path(&job, OutputPanelMode::Stdout),
        Some(PathBuf::from("/tmp/stdout.log"))
    );
    assert_eq!(
        watched_output_path(&job, OutputPanelMode::Stderr),
        Some(PathBuf::from("/tmp/stderr.log"))
    );
    assert_eq!(watched_output_path(&job, OutputPanelMode::Workdir), None);
    assert_eq!(watched_output_path(&job, OutputPanelMode::Collapsed), None);
}

#[test]
fn output_title_omits_scroll_indicators() {
    let mut app = test_app(1, Some(0));
    app.set_focus(Focus::Log);
    app.set_job_output("line 1\nline 2\nline 3\n");
    app.set_output_mode(OutputPanelMode::Stdout);
    app.set_output_anchor(ScrollAnchor::Top);
    app.set_output_offset(12);

    let text = buffer_text(&draw_app(&mut app, 100, 29), 100, 29);

    assert!(text.contains("Stdout"));
    assert!(!text.contains("B-"));
    assert!(!text.contains("[tail]"));
    assert!(!text.contains("[scroll]"));
}

#[test]
fn help_line_shows_global_output_switch() {
    let mut resources_app = test_app(3, Some(0));
    resources_app.set_focus(Focus::Resources);
    let resources_text = buffer_text(&draw_app(&mut resources_app, 120, 12), 120, 12);
    assert!(!resources_text.contains("mode:"));
    assert!(resources_text.contains("⇥ toggle"));
    assert!(!resources_text.contains("focus"));
    assert!(!resources_text.contains("↑/↓: move"));
    assert!(!resources_text.contains("j/k"));
    assert!(!resources_text.contains("d: cancel"));
    assert!(!resources_text.contains("r: rename"));

    let mut jobs_app = test_app(3, Some(0));
    let jobs_text = buffer_text(&draw_app(&mut jobs_app, 120, 21), 120, 21);
    assert!(!jobs_text.contains("mode:"));
    assert!(jobs_text.contains("⌃c path"));
    assert!(jobs_text.contains("⌃r rename"));
    assert!(jobs_text.contains("d detail"));

    let mut details_app = test_app(3, Some(0));
    details_app.set_focus(Focus::Details);
    let details_text = buffer_text(&draw_app(&mut details_app, 120, 12), 120, 12);
    assert!(!details_text.contains("mode:"));
    assert!(details_text.contains("⇥ toggle"));

    let mut output_app = test_app(3, Some(0));
    output_app.set_focus(Focus::Log);
    let output_text = buffer_text(&draw_app(&mut output_app, 120, 12), 120, 12);
    assert!(!output_text.contains("mode:"));
    assert!(output_text.contains("⇥ toggle"));
    assert!(!output_text.contains("↑/↓: scroll"));
    assert!(!output_text.contains("j/k"));

    let mut collapsed_app = test_app(3, Some(0));
    collapsed_app.set_output_mode(OutputPanelMode::Collapsed);
    let collapsed_text = buffer_text(&draw_app(&mut collapsed_app, 120, 12), 120, 12);
    assert!(!collapsed_text.contains("mode:"));
    assert!(collapsed_text.contains("⇥ toggle"));
}

#[test]
fn log_focus_routes_navigation_to_log_scrolling() {
    let mut app = test_app(30, Some(2));
    let _ = draw_app(&mut app, 120, 29);
    app.set_output_mode(OutputPanelMode::Stdout);
    app.set_focus(Focus::Log);
    app.set_job_output(
        (0..32)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    app.set_output_anchor(ScrollAnchor::Top);
    app.set_output_offset(0);

    app.handle(AppMessage::Key(key('j')));
    assert_eq!(app.selected_job_index(), Some(2));
    assert_eq!(app.output_offset(), 1);

    app.handle(AppMessage::Key(special_key(KeyCode::PageDown)));
    assert!(app.output_offset() > 1);

    app.handle(AppMessage::Key(key('G')));
    assert_eq!(app.output_anchor(), ScrollAnchor::Bottom);
    assert_eq!(app.output_offset(), 0);

    app.handle(AppMessage::Key(key('g')));
    assert_eq!(app.output_anchor(), ScrollAnchor::Top);
    assert_eq!(app.output_offset(), 0);

    app.handle(AppMessage::Key(key('w')));
    assert!(app.output_wrap());
}

#[test]
fn output_scroll_clamps_at_file_start_and_end() {
    let mut app = test_app(1, Some(0));
    let _ = draw_app(&mut app, 120, 29);
    app.set_focus(Focus::Log);
    app.set_job_output(
        (0..32)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    app.set_output_anchor(ScrollAnchor::Top);
    app.set_output_offset(0);
    app.handle(AppMessage::Key(key('k')));
    assert_eq!(app.output_offset(), 0);

    for _ in 0..u16::MAX {
        app.handle(AppMessage::Key(special_key(KeyCode::PageDown)));
    }
    let max_offset = app.max_output_offset();
    assert_eq!(app.output_offset(), max_offset);

    app.set_output_anchor(ScrollAnchor::Bottom);
    app.set_output_offset(0);
    app.handle(AppMessage::Key(key('j')));
    assert_eq!(app.output_offset(), 0);
}
