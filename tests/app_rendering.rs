mod common;
use common::*;

#[test]
fn jobs_table_header_is_bold_with_blue_shortcuts_and_sort_slots() {
    let mut app = test_app(2, Some(0));
    let buffer = draw_app(&mut app, 120, 12);
    let header_y = app.job_list_area().y.saturating_add(1);
    let header_text = row_text(&buffer, app.job_list_area(), header_y);

    for label in ["st", "partition", "jobid", "name", "user", "time"] {
        assert!(
            header_text.contains(label),
            "missing header label {label}: {header_text}"
        );
    }

    let blue_header_symbols = (app.job_list_area().x
        ..app
            .job_list_area()
            .x
            .saturating_add(app.job_list_area().width))
        .filter_map(|x| {
            let cell = &buffer[(x, header_y)];
            (cell.fg == Color::Blue).then(|| cell.symbol().to_string())
        })
        .collect::<Vec<_>>();

    assert_eq!(blue_header_symbols, vec!["s", "p", "j", "n", "u", "t"]);
    assert!(
        (app.job_list_area().x..app.job_list_area().right()).all(|x| !buffer[(x, header_y)]
            .style()
            .add_modifier
            .contains(Modifier::UNDERLINED))
    );
    for label in ["st", "partition", "jobid", "name", "user", "time"] {
        let byte_index = header_text.find(label).unwrap();
        let start = app.job_list_area().x + header_text[..byte_index].chars().count() as u16;
        assert!(
            (start..start + label.len() as u16).all(|x| {
                buffer[(x, header_y)]
                    .style()
                    .add_modifier
                    .contains(Modifier::BOLD)
            }),
            "not bold: {label} at {start}"
        );
    }
    let indicator_byte_index = header_text.find('▲').unwrap();
    let indicator_x =
        app.job_list_area().x + header_text[..indicator_byte_index].chars().count() as u16;
    assert!(
        buffer[(indicator_x, header_y)]
            .style()
            .add_modifier
            .contains(Modifier::BOLD)
    );
    assert!(header_text.contains("st  "));
    assert!(header_text.contains("partition  "));
    assert!(header_text.contains("jobid  "));
    assert!(header_text.contains("name  "));
    assert!(header_text.contains("user  "));
}

#[test]
fn job_name_column_is_capped_at_25_characters() {
    let mut app = test_app(1, Some(0));
    app.jobs_mut()[0].name = "1234567890123456789012345overflow".to_string();
    let buffer = draw_app(&mut app, 120, 20);
    let header_y = app.job_list_area().y.saturating_add(1);
    let row_y = app.job_list_area().y.saturating_add(2);
    let header = row_text(&buffer, app.job_list_area(), header_y);
    let row = row_text(&buffer, app.job_list_area(), row_y);

    assert_eq!(
        header.find("user").unwrap() - header.find("name").unwrap(),
        26
    );
    assert_eq!(
        header.find("time").unwrap() - header.find("user").unwrap(),
        6
    );
    assert!(row.contains("1234567890123456789012345"));
    assert!(!row.contains("overflow"));
}

#[test]
fn completed_job_rows_are_dimmed() {
    let mut app = test_app(2, Some(0));
    app.jobs_mut()[1].state_compact = "CD".to_string();
    let buffer = draw_app(&mut app, 120, 20);
    let area = app.job_list_area();
    let completed_y = (area.y + 2..area.bottom())
        .find(|&y| row_text(&buffer, area, y).contains("CD"))
        .unwrap();

    assert!((area.x + 2..area.right().saturating_sub(2)).all(|x| {
        buffer[(x, completed_y)]
            .style()
            .add_modifier
            .contains(Modifier::DIM)
    }));
}

#[test]
fn table_content_has_one_cell_of_horizontal_padding() {
    let mut app = test_app(2, Some(0));
    let buffer = draw_app(&mut app, 120, 20);

    let jobs = app.job_list_area();
    assert_eq!(buffer[(jobs.x + 1, jobs.y + 1)].symbol(), " ");
    assert_eq!(buffer[(jobs.right() - 2, jobs.y + 1)].symbol(), " ");

    let resources = app.resource_area();
    assert_eq!(buffer[(resources.x + 1, resources.y + 1)].symbol(), " ");
    assert_eq!(
        buffer[(resources.right() - 2, resources.y + 1)].symbol(),
        " "
    );
}

#[test]
fn resources_and_job_details_share_the_top_row() {
    let mut app = test_app(3, Some(0));
    let _ = draw_app(&mut app, 100, 40);

    assert_eq!(app.resource_area(), Rect::new(0, 0, 35, 28));
    assert_eq!(app.job_list_area().y, 0);
    assert!(app.job_details_area().y > app.job_list_area().y);
    assert_eq!(app.job_list_area().x, 34);
    assert_eq!(app.job_list_area().width, 66);
    assert!(app.job_details_area().x > app.job_list_area().x);
    assert_eq!(app.job_output_area().x, 0);
    assert_eq!(app.job_output_area().width, 100);
    assert_eq!(
        app.job_output_area().y,
        app.job_list_area().y + app.job_list_area().height
    );
    assert!(app.job_output_area().height >= 4);
}

#[test]
fn output_requires_minimum_height_and_top_panel_is_bounded() {
    let mut short = test_app(1, Some(0));
    let _ = draw_app(&mut short, 100, 28);
    assert_eq!(short.job_output_area(), Rect::default());
    assert_eq!(short.job_list_area().height, 26);

    let mut minimum = test_app(1, Some(0));
    let _ = draw_app(&mut minimum, 100, 29);
    assert_eq!(minimum.job_list_area().height, 23);
    assert_eq!(minimum.job_output_area().height, 4);

    let mut large = test_app(1, Some(0));
    let _ = draw_app(&mut large, 100, 50);
    assert_eq!(large.job_list_area().height, 31);
    assert_eq!(large.job_output_area().height, 17);
}

#[test]
fn top_panels_share_a_single_table_border() {
    let mut app = test_app(3, Some(0));
    let buffer = draw_app(&mut app, 100, 40);
    let seam_x = app.job_details_area().x;
    let bottom_y = app.job_details_area().y + app.job_details_area().height - 1;

    assert_eq!(buffer[(seam_x, app.job_details_area().y)].symbol(), "╭");
    assert_eq!(buffer[(seam_x, app.job_details_area().y + 1)].symbol(), "│");
    assert_eq!(buffer[(seam_x, bottom_y)].symbol(), "╰");
    assert_eq!(
        buffer[(app.job_list_area().x, app.job_list_area().y)].symbol(),
        "┬"
    );
    assert_eq!(
        buffer[(app.job_output_area().x, app.job_output_area().y)].symbol(),
        "╭"
    );
}

#[test]
fn default_output_mode_is_workdir() {
    let app = test_app(1, Some(0));

    assert_eq!(app.output_mode(), OutputPanelMode::Workdir);
}

#[test]
fn o_cycles_output_modes_in_order() {
    let mut app = test_app(1, Some(0));
    let _ = draw_app(&mut app, 120, 40);

    for expected_mode in [
        OutputPanelMode::Stdout,
        OutputPanelMode::Stderr,
        OutputPanelMode::Collapsed,
        OutputPanelMode::Workdir,
    ] {
        app.handle(AppMessage::Key(key('o')));
        assert_eq!(app.output_mode(), expected_mode);
    }
}

#[test]
fn tab_cycles_output_modes_in_order() {
    let mut app = test_app(1, Some(0));
    let _ = draw_app(&mut app, 120, 40);

    for expected_mode in [
        OutputPanelMode::Stdout,
        OutputPanelMode::Stderr,
        OutputPanelMode::Collapsed,
        OutputPanelMode::Workdir,
    ] {
        app.handle(AppMessage::Key(special_key(KeyCode::Tab)));
        assert_eq!(app.output_mode(), expected_mode);
    }
}

#[test]
fn o_switches_output_mode_from_every_focus() {
    for focus in [Focus::Resources, Focus::Jobs, Focus::Details, Focus::Log] {
        let mut app = test_app(1, Some(0));
        app.set_focus(focus);
        let _ = draw_app(&mut app, 120, 40);

        app.handle(AppMessage::Key(key('o')));

        assert_eq!(
            app.output_mode(),
            OutputPanelMode::Stdout,
            "focus {focus:?}"
        );
    }
}

#[test]
fn o_does_not_cycle_while_filter_dialog_is_open() {
    let mut app = test_app(1, Some(0));
    open_filter(&mut app);

    app.handle(AppMessage::Key(key('o')));

    assert_eq!(app.output_mode(), OutputPanelMode::Workdir);
    match app.dialog() {
        Some(Dialog::FilterJobs { input }) => assert_eq!(input.value(), "o"),
        _ => panic!("expected filter dialog"),
    }
}

#[test]
fn collapsed_layout_hides_output_panel_and_expands_jobs() {
    let mut app = test_app(3, Some(0));
    app.set_output_mode(OutputPanelMode::Collapsed);

    let text = buffer_text(&draw_app(&mut app, 100, 12), 100, 12);

    assert_eq!(app.resource_area().width, 35);
    assert_eq!(app.job_list_area().x, 34);
    assert_eq!(app.job_list_area().width, 66);
    assert_eq!(app.job_list_area().height, 11);
    assert_eq!(app.job_output_area(), Rect::default());
    assert!(!text.contains("Stdout"));
    assert!(!text.contains("Stderr"));
    assert!(!text.contains("Workdir view not implemented yet"));
}

#[test]
fn selected_job_row_renders_below_header() {
    let mut app = test_app(4, Some(0));
    let buffer = draw_app(&mut app, 120, 16);
    let header_y = app.job_list_area().y.saturating_add(1);
    let first_row_y = app.job_list_area().y.saturating_add(2);
    let header_text = row_text(&buffer, app.job_list_area(), header_y);
    let first_row_text = row_text(&buffer, app.job_list_area(), first_row_y);

    assert!(header_text.contains("partition"));
    assert!(first_row_text.contains("job-0"));

    let job_name_x = app.job_list_area().x + first_row_text.find("job-0").unwrap() as u16;
    assert_eq!(
        buffer[(job_name_x, first_row_y)].style().bg,
        Some(Color::Green)
    );
}

#[test]
fn selected_job_highlight_is_only_visible_while_jobs_are_focused() {
    let mut app = test_app(4, Some(0));
    let focused = draw_app(&mut app, 120, 40);
    let row_y = app.job_list_area().y.saturating_add(2);
    let job_x = app.job_list_area().x.saturating_add(2);
    assert_eq!(focused[(job_x, row_y)].bg, Color::Green);

    app.handle(AppMessage::Key(special_key(KeyCode::Left)));
    let unfocused = draw_app(&mut app, 120, 40);
    assert_eq!(app.focus(), Focus::Resources);
    assert_ne!(unfocused[(job_x, row_y)].bg, Color::Green);
    assert_eq!(app.selected_job_index(), Some(0));
}

#[test]
fn panel_titles_keep_one_leading_border_cell_and_trailing_padding() {
    let mut app = test_app(3, Some(0));
    let buffer = draw_app(&mut app, 120, 40);
    let text = buffer_text(&buffer, 120, 40);

    for title in ["─ Resources ", "─ Jobs (3) ", "─ Workdir "] {
        assert!(text.contains(title), "missing title {title:?}");
    }

    for (area, corner) in [
        (app.resource_area(), "╭"),
        (app.job_list_area(), "┬"),
        (app.job_output_area(), "╭"),
    ] {
        assert_eq!(buffer[(area.x, area.y)].symbol(), corner);
        assert_eq!(buffer[(area.x + 1, area.y)].symbol(), "─");
        assert_eq!(buffer[(area.x + 1, area.y)].fg, Color::DarkGray);
    }
}

#[test]
fn output_title_is_the_active_mode() {
    for (mode, title) in [
        (OutputPanelMode::Workdir, "─ Workdir "),
        (OutputPanelMode::Stdout, "─ Stdout "),
        (OutputPanelMode::Stderr, "─ Stderr "),
    ] {
        let mut app = test_app(1, Some(0));
        app.set_output_mode(mode);
        let text = buffer_text(&draw_app(&mut app, 120, 40), 120, 40);
        assert!(text.contains(title), "missing title {title:?}");
        assert!(!text.contains("Outputs ("));
    }
}

#[test]
fn jobs_scrollbar_threshold_uses_only_body_rows() {
    let mut measuring_app = test_app(0, None);
    let _ = draw_app(&mut measuring_app, 120, 20);
    let visible_body_rows = usize::from(measuring_app.job_list_height());

    let mut app_without_overflow = test_app(visible_body_rows, Some(0));
    let buffer_without_overflow = draw_app(&mut app_without_overflow, 120, 20);
    let symbols_without_overflow = jobs_area_symbols(
        &buffer_without_overflow,
        app_without_overflow.job_list_rows_area(),
    );
    assert!(
        !symbols_without_overflow
            .iter()
            .any(|symbol| symbol == VERTICAL_SCROLLBAR_THUMB)
    );

    let mut app_with_overflow = test_app(visible_body_rows.saturating_add(1), Some(0));
    let buffer_with_overflow = draw_app(&mut app_with_overflow, 120, 20);
    let symbols_with_overflow = jobs_area_symbols(
        &buffer_with_overflow,
        app_with_overflow.job_list_rows_area(),
    );
    assert!(
        symbols_with_overflow
            .iter()
            .any(|symbol| symbol == VERTICAL_SCROLLBAR_THUMB)
    );
}

#[test]
fn jobs_half_page_up_uses_body_row_height() {
    let mut app = test_app(30, Some(20));
    let _ = draw_app(&mut app, 120, 20);
    let body_rows = app.job_list_height();
    let start = 20usize;

    app.scroll_jobs_half_page_up();
    assert_eq!(
        app.selected_job_index(),
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

    assert!(app_scrolled.job_list_offset() > app_at_top.job_list_offset());
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
fn focused_border_style_tracks_the_active_panel() {
    let mut jobs_app = test_app(3, Some(0));
    let jobs_buffer = draw_app(&mut jobs_app, 120, 20);
    assert_ne!(
        border_fg(&jobs_buffer, jobs_app.job_list_area()),
        Some(Color::Green)
    );
    assert_ne!(
        border_fg(&jobs_buffer, details_area(&jobs_app)),
        Some(Color::Green)
    );
    assert_ne!(
        border_fg(&jobs_buffer, jobs_app.job_output_area()),
        Some(Color::Green)
    );

    let mut details_app = test_app(3, Some(0));
    details_app.set_focus(Focus::Details);
    let details_buffer = draw_app(&mut details_app, 120, 20);
    assert_ne!(
        border_fg(&details_buffer, details_app.job_list_area()),
        Some(Color::Green)
    );
    assert_ne!(
        border_fg(&details_buffer, details_area(&details_app)),
        Some(Color::Green)
    );
    assert_ne!(
        border_fg(&details_buffer, details_app.job_output_area()),
        Some(Color::Green)
    );

    let mut log_app = test_app(3, Some(0));
    log_app.set_focus(Focus::Log);
    let log_buffer = draw_app(&mut log_app, 120, 20);
    assert_ne!(
        border_fg(&log_buffer, log_app.job_list_area()),
        Some(Color::Green)
    );
    assert_ne!(
        border_fg(&log_buffer, details_area(&log_app)),
        Some(Color::Green)
    );
    assert_ne!(
        border_fg(&log_buffer, log_app.job_output_area()),
        Some(Color::Green)
    );
}

#[test]
fn top_panels_have_rounded_gray_borders() {
    let mut app = test_app(3, Some(0));
    let buffer = draw_app(&mut app, 120, 40);
    let resources = app.resource_area();
    let jobs = app.job_list_area();
    let details = details_area(&app);
    let output = app.job_output_area();

    for (position, symbol) in [
        ((resources.x, resources.y), "╭"),
        ((jobs.x, jobs.y), "┬"),
        ((jobs.right() - 1, jobs.y), "╮"),
        ((details.x, details.y), "╭"),
        ((details.right() - 1, details.y), "╮"),
        ((output.x, output.y), "╭"),
        ((output.right() - 1, output.y), "╮"),
    ] {
        assert_eq!(buffer[position].symbol(), symbol);
        assert_eq!(buffer[position].fg, Color::DarkGray);
    }
}

#[test]
fn collapsed_mode_keeps_focus_highlight_off_hidden_output_panel() {
    let mut app = test_app(3, Some(0));
    app.set_output_mode(OutputPanelMode::Collapsed);
    app.set_focus(Focus::Jobs);

    let buffer = draw_app(&mut app, 120, 20);

    assert_eq!(app.job_output_area(), Rect::default());
    assert_ne!(border_fg(&buffer, app.job_list_area()), Some(Color::Green));
}

#[test]
fn help_line_uses_static_global_shortcuts() {
    let mut jobs_app = test_app(3, Some(0));
    let jobs_text = buffer_text(&draw_app(&mut jobs_app, 120, 21), 120, 21);
    assert!(!jobs_text.contains("mode:"));
    assert!(jobs_text.contains("⌃r rename"));
    assert!(jobs_text.contains("⇥ toggle"));
    assert!(jobs_text.contains("↵ cd"));
    assert!(jobs_text.contains("d detail"));
    assert!(!jobs_text.contains("focus"));

    jobs_app = copyable_jobs_app();
    jobs_app.handle(AppMessage::Key(key('c')));
    let copy_text = buffer_text(&draw_app(&mut jobs_app, 120, 20), 120, 20);
    assert!(!copy_text.contains("mode:"));
    assert!(copy_text.contains("c: dir-name"));
    assert!(copy_text.contains("d: dir-url"));

    let mut details_app = test_app(3, Some(0));
    details_app.set_focus(Focus::Details);
    let details_text = buffer_text(&draw_app(&mut details_app, 120, 12), 120, 12);
    assert!(!details_text.contains("mode:"));
    assert!(details_text.contains("⇥ toggle"));

    details_app.set_output_mode(OutputPanelMode::Workdir);
    let workdir_text = buffer_text(&draw_app(&mut details_app, 120, 12), 120, 12);
    assert!(workdir_text.contains("↵ cd"));

    let mut log_app = test_app(3, Some(0));
    log_app.set_focus(Focus::Log);
    let log_text = buffer_text(&draw_app(&mut log_app, 120, 12), 120, 12);
    assert!(!log_text.contains("mode:"));
    assert!(log_text.contains("⇥ toggle"));
    assert!(!log_text.contains("↑/↓: scroll"));
}

#[test]
fn help_keys_are_blue_and_descriptions_are_gray() {
    let mut app = test_app(3, Some(0));
    let buffer = draw_app(&mut app, 120, 21);

    for (x, y) in [
        (0, 19),
        (1, 19),
        (12, 19),
        (13, 19),
        (24, 19),
        (25, 19),
        (38, 19),
        (39, 19),
        (0, 20),
        (11, 20),
        (18, 20),
        (29, 20),
        (40, 20),
    ] {
        assert_eq!(buffer[(x, y)].fg, Color::Blue, "key at ({x}, {y})");
    }
    for (x, y) in [
        (2, 19),
        (14, 19),
        (26, 19),
        (40, 19),
        (1, 20),
        (12, 20),
        (19, 20),
        (30, 20),
        (41, 20),
    ] {
        assert_eq!(buffer[(x, y)].fg, Color::DarkGray, "text at ({x}, {y})");
    }
}
