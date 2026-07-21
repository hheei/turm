mod common;
use common::*;

#[test]
fn bracket_keys_do_not_switch_focus() {
    let mut app = test_app(3, Some(0));

    assert_eq!(app.focus(), Focus::Jobs);

    app.handle(AppMessage::Key(key('[')));
    app.handle(AppMessage::Key(key(']')));
    assert_eq!(app.focus(), Focus::Jobs);
}

#[test]
fn arrows_cycle_panel_focus_and_tab_cycles_output() {
    let mut app = test_app(3, Some(0));
    let _ = draw_app(&mut app, 120, 40);

    assert_eq!(app.output_mode(), OutputPanelMode::Workdir);
    app.handle(AppMessage::Key(special_key(KeyCode::Tab)));
    assert_eq!(app.output_mode(), OutputPanelMode::Stdout);

    app.handle(AppMessage::Key(special_key(KeyCode::Right)));
    assert_eq!(app.focus(), Focus::Log);
    app.handle(AppMessage::Key(special_key(KeyCode::Right)));
    assert_eq!(app.focus(), Focus::Resources);

    app.handle(AppMessage::Key(special_key(KeyCode::Tab)));
    assert_eq!(app.output_mode(), OutputPanelMode::Stderr);
    app.handle(AppMessage::Key(special_key(KeyCode::Right)));
    assert_eq!(app.focus(), Focus::Jobs);
    app.handle(AppMessage::Key(special_key(KeyCode::Left)));
    assert_eq!(app.focus(), Focus::Resources);
}

#[test]
fn tab_closing_focused_output_returns_focus_to_jobs() {
    let mut app = test_app(3, Some(0));
    let _ = draw_app(&mut app, 120, 40);
    app.set_output_mode(OutputPanelMode::Stderr);
    app.set_focus(Focus::Log);

    app.handle(AppMessage::Key(special_key(KeyCode::Tab)));

    assert_eq!(app.output_mode(), OutputPanelMode::Collapsed);
    assert_eq!(app.focus(), Focus::Jobs);
}

#[test]
fn filter_dialog_keeps_bracket_input_and_focus() {
    let mut app = test_app(3, Some(0));
    open_filter(&mut app);

    app.handle(AppMessage::Key(key(']')));
    app.handle(AppMessage::Key(key('[')));

    assert_eq!(app.focus(), Focus::Jobs);
    match app.dialog() {
        Some(Dialog::FilterJobs { input }) => assert_eq!(input.value(), "]["),
        _ => panic!("expected filter dialog"),
    }
}

#[test]
fn jobs_navigation_stays_in_jobs_focus() {
    let mut app = test_app(4, Some(0));

    app.handle(AppMessage::Key(special_key(KeyCode::Down)));
    assert_eq!(app.selected_job_index(), Some(1));

    app.handle(AppMessage::Key(key('k')));
    assert_eq!(app.selected_job_index(), Some(0));
}

#[test]
fn enter_opens_output_file_or_changes_to_workdir() {
    let mut editor_app = test_app(1, Some(0));
    editor_app.set_output_mode(OutputPanelMode::Stdout);
    editor_app.jobs_mut()[0].stdout = Some(PathBuf::from("/tmp/stdout.log"));
    editor_app.handle(AppMessage::Key(special_key(KeyCode::Enter)));
    assert_eq!(
        editor_app.pending_exit(),
        Some(&AppExit::OpenEditor(PathBuf::from("/tmp/stdout.log")))
    );

    let mut workdir_app = test_app(1, Some(0));
    workdir_app.set_output_mode(OutputPanelMode::Collapsed);
    workdir_app.jobs_mut()[0].workdir = Some(PathBuf::from("/tmp/workdir"));
    workdir_app.handle(AppMessage::Key(special_key(KeyCode::Enter)));
    assert_eq!(
        workdir_app.pending_exit(),
        Some(&AppExit::ChangeDirectory(PathBuf::from("/tmp/workdir")))
    );
}

#[test]
fn mouse_click_focuses_jobs_and_output_panels() {
    let mut app = test_app(4, Some(0));
    let _ = draw_app(&mut app, 120, 40);

    let details = details_area(&app);
    app.handle(AppMessage::MouseClick {
        column: details.x.saturating_add(1),
        row: details.y.saturating_add(1),
    });
    assert_eq!(app.focus(), Focus::Jobs);

    let log = app.job_output_area();
    app.handle(AppMessage::MouseClick {
        column: log.x.saturating_add(1),
        row: log.y.saturating_add(1),
    });
    assert_eq!(app.focus(), Focus::Log);
}

#[test]
fn mouse_click_on_workdir_selects_clicked_entry() {
    let mut app = test_app(1, Some(0));
    app.set_output_mode(OutputPanelMode::Workdir);
    app.set_workdir_entries(vec![
        WorkdirEntry {
            name: "alpha".to_string(),
            path: PathBuf::from("/tmp/alpha"),
            kind: WorkdirEntryKind::Directory,
        },
        WorkdirEntry {
            name: "beta".to_string(),
            path: PathBuf::from("/tmp/beta"),
            kind: WorkdirEntryKind::File,
        },
        WorkdirEntry {
            name: "gamma".to_string(),
            path: PathBuf::from("/tmp/gamma"),
            kind: WorkdirEntryKind::File,
        },
        WorkdirEntry {
            name: "delta".to_string(),
            path: PathBuf::from("/tmp/delta"),
            kind: WorkdirEntryKind::File,
        },
    ]);
    app.set_workdir_selected(Some(0));
    app.set_workdir_offset(1);

    let _ = draw_app(&mut app, 80, 40);
    let viewport = app.layout().viewport;

    app.handle(AppMessage::MouseClick {
        column: viewport.x.saturating_add(1),
        row: viewport.y.saturating_add(2),
    });

    assert_eq!(app.focus(), Focus::Log);
    assert_eq!(app.workdir_selected(), Some(2));
}

#[test]
fn mouse_click_on_empty_workdir_row_only_changes_focus() {
    let mut app = test_app(1, Some(0));
    app.set_output_mode(OutputPanelMode::Workdir);
    app.set_workdir_entries(vec![
        WorkdirEntry {
            name: "alpha".to_string(),
            path: PathBuf::from("/tmp/alpha"),
            kind: WorkdirEntryKind::Directory,
        },
        WorkdirEntry {
            name: "beta".to_string(),
            path: PathBuf::from("/tmp/beta"),
            kind: WorkdirEntryKind::File,
        },
    ]);
    app.set_workdir_selected(Some(1));

    let _ = draw_app(&mut app, 80, 40);
    let viewport = app.layout().viewport;
    let empty_row = viewport
        .y
        .saturating_add(app.workdir_entry_count() as u16 + 1);
    assert!(empty_row < viewport.y.saturating_add(viewport.height));

    app.handle(AppMessage::MouseClick {
        column: viewport.x.saturating_add(1),
        row: empty_row,
    });

    assert_eq!(app.focus(), Focus::Log);
    assert_eq!(app.workdir_selected(), Some(1));
    assert_eq!(app.workdir_offset(), 0);
}

#[test]
fn mouse_click_on_workdir_selects_the_clicked_entry() {
    let mut app = test_app(1, Some(0));
    app.set_output_mode(OutputPanelMode::Workdir);
    app.set_workdir_entries(vec![
        WorkdirEntry {
            name: "alpha".to_string(),
            path: PathBuf::from("/tmp/alpha"),
            kind: WorkdirEntryKind::Directory,
        },
        WorkdirEntry {
            name: "beta".to_string(),
            path: PathBuf::from("/tmp/beta"),
            kind: WorkdirEntryKind::File,
        },
    ]);
    app.set_workdir_selected(Some(0));
    let _ = draw_app(&mut app, 80, 40);
    let viewport = app.layout().viewport;
    let row = viewport.y + 2;

    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: viewport.x + 1,
        row,
        modifiers: KeyModifiers::NONE,
    }));
    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: viewport.x + 1,
        row,
        modifiers: KeyModifiers::NONE,
    }));

    assert_eq!(app.focus(), Focus::Log);
    assert_eq!(app.workdir_selected(), Some(1));
    assert_eq!(app.pending_clipboard_copy(), None);
}

#[test]
fn mouse_click_on_jobs_restores_jobs_focus_and_selection() {
    let mut app = test_app(6, Some(1));
    let buffer = draw_app(&mut app, 120, 20);
    app.set_focus(Focus::Log);

    let first_row_y = app.job_list_area().y.saturating_add(2);
    let first_row_text = row_text(&buffer, app.job_list_area(), first_row_y);
    let job_name_x = app.job_list_area().x + first_row_text.find("job-0").unwrap() as u16;

    app.handle(AppMessage::MouseClick {
        column: job_name_x,
        row: first_row_y,
    });

    assert_eq!(app.focus(), Focus::Jobs);
    assert_eq!(app.selected_job_index(), Some(0));
}

#[test]
fn mouse_short_click_selects_a_job_without_highlighting() {
    let mut app = test_app(4, Some(1));
    let buffer = draw_app(&mut app, 120, 20);
    let first_row_y = app.job_list_area().y.saturating_add(2);
    let first_row_text = row_text(&buffer, app.job_list_area(), first_row_y);
    let job_name_x = app.job_list_area().x + first_row_text.find("job-0").unwrap() as u16;

    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: job_name_x,
        row: first_row_y,
        modifiers: KeyModifiers::NONE,
    }));
    let buffer = draw_app(&mut app, 120, 20);

    assert_eq!(app.focus(), Focus::Jobs);
    assert_eq!(app.selected_job_index(), Some(0));
    assert!(!(0..20).any(|y| (0..120).any(|x| buffer[(x, y)].bg == Color::Blue)));

    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: job_name_x,
        row: first_row_y,
        modifiers: KeyModifiers::NONE,
    }));
    assert_eq!(app.pending_clipboard_copy(), None);
}

#[test]
fn dialog_blocks_mouse_focus_changes() {
    let mut app = test_app(4, Some(0));
    let _ = draw_app(&mut app, 120, 40);
    app.set_dialog(Some(Dialog::FilterJobs {
        input: Input::new(String::new()),
    }));

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
    assert_eq!(app.focus(), Focus::Jobs);
}

#[test]
fn mouse_selection_is_clipped_to_one_panel_and_copies_without_borders() {
    let mut app = test_app(4, Some(0));
    let _ = draw_app(&mut app, 120, 20);
    let area = app.job_list_area();
    let down = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: area.x + 2,
        row: area.y + 1,
        modifiers: KeyModifiers::NONE,
    });
    let drag = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: area.x + 25,
        row: area.y + 4,
        modifiers: KeyModifiers::NONE,
    });
    let up = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: area.x + 25,
        row: area.y + 4,
        modifiers: KeyModifiers::NONE,
    });
    app.handle_input_event(down);
    app.handle_input_event(drag);
    app.handle_input_event(up);
    let _ = draw_app(&mut app, 120, 20);

    let copied = app
        .pending_clipboard_copy()
        .expect("mouse up copied selection");
    assert!(!copied.contains('╭'));
    assert!(!copied.starts_with(' '));
    assert!(copied.contains("partition"));
    assert!(copied.contains('\n'));
}

#[test]
fn mouse_selection_is_row_major_and_excludes_the_scrollbar() {
    let mut app = app_with_output_lines(&[
        "abcdefghij",
        "klmnopqrst",
        "uvwxyz0123",
        "456789ABCD",
        "EFGHIJKLMN",
        "OPQRSTUVWX",
        "YZabcdefgh",
        "ijklmnopqr",
    ]);
    app.set_output_anchor(ScrollAnchor::Top);
    let _ = draw_app(&mut app, 80, 29);
    let viewport = app.layout().viewport;
    assert!(app.layout().show_vertical);

    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: viewport.x + 3,
        row: viewport.y,
        modifiers: KeyModifiers::NONE,
    }));
    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: viewport.x + 1,
        row: viewport.y + 1,
        modifiers: KeyModifiers::NONE,
    }));
    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: viewport.x + 1,
        row: viewport.y + 1,
        modifiers: KeyModifiers::NONE,
    }));
    let buffer = draw_app(&mut app, 80, 29);

    let scrollbar_x = viewport.x + viewport.width;
    assert_eq!(
        buffer[(scrollbar_x, viewport.y)].symbol(),
        VERTICAL_SCROLLBAR_THUMB
    );
    assert_ne!(buffer[(scrollbar_x, viewport.y)].bg, Color::Blue);

    assert_eq!(app.pending_clipboard_copy(), Some("abcdefghij\nkl"));
    assert!(buffer_text(&buffer, 80, 29).contains("Copied to clipboard"));
    assert!(!(0..29).any(|y| (0..80).any(|x| buffer[(x, y)].bg == Color::Blue)));
}

#[test]
fn mouse_selection_excludes_output_left_padding() {
    let mut app = app_with_output_lines(&["abcdefghij", "klmnopqrst"]);
    let _ = draw_app(&mut app, 80, 29);
    let area = app.job_output_area();
    let viewport = app.layout().viewport;
    assert_eq!(viewport.x, area.x + 2);

    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: area.x + 1,
        row: viewport.y,
        modifiers: KeyModifiers::NONE,
    }));
    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: viewport.x + 3,
        row: viewport.y + 1,
        modifiers: KeyModifiers::NONE,
    }));
    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: viewport.x + 3,
        row: viewport.y + 1,
        modifiers: KeyModifiers::NONE,
    }));

    assert_eq!(app.pending_clipboard_copy(), None);
}

#[test]
fn details_selection_is_aware_of_wrapped_key_and_value_elements() {
    let mut app = test_app(1, Some(0));
    app.jobs_mut()[0].tres =
        "cpu=64,mem=512G,gres/gpu=8,node=4,billing=64,license=vasp".to_string();
    let buffer = draw_app(&mut app, 120, 20);
    let details = app.job_details_area();
    let first_row = (details.y + 1..details.bottom() - 1)
        .find(|&y| row_text(&buffer, details, y).contains("TRES"))
        .unwrap();
    let key_x = details.x + 2;
    let value_x = key_x + 9;

    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: key_x,
        row: first_row,
        modifiers: KeyModifiers::NONE,
    }));
    app.handle_input_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: value_x + 2,
        row: first_row,
        modifiers: KeyModifiers::NONE,
    }));
    let selected = draw_app(&mut app, 120, 20);

    assert_eq!(selected[(key_x, first_row + 1)].bg, Color::Blue);
    assert_ne!(selected[(value_x, first_row + 1)].bg, Color::Blue);
}
