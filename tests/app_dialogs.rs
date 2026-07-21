mod common;
use common::*;

#[test]
fn ctrl_t_still_opens_time_limit_dialog() {
    let mut app = test_app(2, Some(0));

    app.handle(AppMessage::Key(key_with_modifiers(
        't',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert!(matches!(app.dialog(), Some(Dialog::EditTimeLimit { .. })));
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
fn ctrl_d_in_jobs_focus_opens_cancel_confirmation_popup() {
    let mut app = filtered_jobs_app();

    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    match app.dialog() {
        Some(Dialog::ConfirmCancelJob {
            id,
            name,
            signal,
            selected,
            ..
        }) => {
            assert_eq!(id, "101512");
            assert_eq!(name, "vasp-alpha");
            assert_eq!(*signal, None);
            assert_eq!(*selected, ConfirmCancelChoice::No);
        }
        _ => panic!("expected confirm cancel dialog"),
    }
}

#[test]
fn d_toggles_details_panel() {
    let mut app = filtered_jobs_app();
    app.set_focus(Focus::Details);

    assert!(app.details_visible());
    app.handle(AppMessage::Key(key('d')));

    assert!(!app.details_visible());
}

#[test]
fn ctrl_d_in_log_focus_opens_cancel_popup() {
    let mut app = app_with_output_lines(&[
        "line 1", "line 2", "line 3", "line 4", "line 5", "line 6", "line 7", "line 8", "line 9",
        "line 10", "line 11", "line 12",
    ]);
    let _ = draw_app(&mut app, 40, 10);
    app.set_focus(Focus::Log);
    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert!(matches!(
        app.dialog(),
        Some(Dialog::ConfirmCancelJob { .. })
    ));
}

#[test]
fn d_in_resources_focus_toggles_details_without_cancel_popup() {
    let mut app = filtered_jobs_app();
    app.set_focus(Focus::Resources);

    let before = app.details_visible();
    app.handle(AppMessage::Key(key('d')));

    assert!(app.dialog().is_none());
    assert_ne!(app.details_visible(), before);
}

#[test]
fn c_opens_copy_popup_and_uppercase_c_stays_unbound() {
    let mut app = copyable_jobs_app();

    app.handle(AppMessage::Key(key('c')));
    assert!(matches!(
        app.dialog(),
        Some(Dialog::CopyJobOutputDirectory { .. })
    ));

    app.set_dialog(None);
    app.handle(AppMessage::Key(key('C')));
    assert!(app.dialog().is_none());
}

#[test]
fn copy_popup_renders_directory_actions() {
    let mut app = copyable_jobs_app();
    app.handle(AppMessage::Key(key('c')));

    let text = buffer_text(&draw_app(&mut app, 120, 20), 120, 20);
    assert!(text.contains("─ Copy ─"));
    assert!(text.contains("copy dir url"));
    assert!(text.contains("/scratch/chlo/vasp-alpha"));
    assert!(text.contains("copy directory name"));
    assert!(text.contains("vasp-alpha"));
}

#[test]
fn copy_popup_copies_directory_name_with_c() {
    let mut app = copyable_jobs_app();
    app.handle(AppMessage::Key(key('c')));
    app.handle(AppMessage::Key(key('c')));

    assert!(app.dialog().is_none());
    assert_eq!(app.pending_clipboard_copy(), Some("vasp-alpha"));
}

#[test]
fn copy_popup_preserves_special_characters_in_directory_path() {
    let mut app = copyable_jobs_app_with_output_path(
        "/scratch/chlo/vasp+alpha %beta#gamma?delta&epsilon/stdout.log",
    );
    app.handle(AppMessage::Key(key('c')));
    app.handle(AppMessage::Key(key('d')));

    assert!(app.dialog().is_none());
    assert_eq!(
        app.pending_clipboard_copy(),
        Some("/scratch/chlo/vasp+alpha %beta#gamma?delta&epsilon")
    );
}

#[test]
fn copy_popup_copies_directory_url_with_d() {
    let mut app = copyable_jobs_app();
    app.handle(AppMessage::Key(key('c')));
    app.handle(AppMessage::Key(key('d')));

    assert!(app.dialog().is_none());
    assert_eq!(
        app.pending_clipboard_copy(),
        Some("/scratch/chlo/vasp-alpha")
    );
}

#[test]
fn ctrl_c_without_selection_does_not_open_copy_popup() {
    let mut app = copyable_jobs_app();
    app.handle(AppMessage::Key(key_with_modifiers(
        'c',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert!(app.dialog().is_none());
}

#[test]
fn ctrl_d_in_log_focus_opens_cancel_popup_again() {
    let mut app = filtered_jobs_app();
    app.set_focus(Focus::Log);

    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert!(matches!(
        app.dialog(),
        Some(Dialog::ConfirmCancelJob { .. })
    ));
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
    assert!(text.contains("[N]o"));
    assert!(text.contains("[Y]es"));
}

#[test]
fn cancel_confirmation_defaults_to_no_and_enter_closes_safely() {
    let mut app = filtered_jobs_app();
    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));
    app.handle(AppMessage::Key(special_key(KeyCode::Enter)));
    assert!(app.dialog().is_none());
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
        assert!(app.dialog().is_none());
    }
}

#[test]
fn cancel_confirmation_arrow_keys_change_selection() {
    let mut app = filtered_jobs_app();
    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));
    app.handle(AppMessage::Key(special_key(KeyCode::Right)));

    match app.dialog() {
        Some(Dialog::ConfirmCancelJob { selected, .. }) => {
            assert_eq!(*selected, ConfirmCancelChoice::Yes)
        }
        _ => panic!("expected confirm cancel dialog"),
    }

    app.handle(AppMessage::Key(special_key(KeyCode::Left)));
    match app.dialog() {
        Some(Dialog::ConfirmCancelJob { selected, .. }) => {
            assert_eq!(*selected, ConfirmCancelChoice::No)
        }
        _ => panic!("expected confirm cancel dialog"),
    }
}

#[test]
fn cancel_confirmation_maps_yes_keys_without_running_scancel() {
    use turm::test_support::CancelConfirmationAction;

    for key_event in [key('y'), key('Y')] {
        assert_eq!(
            cancel_confirmation_action(key_event, ConfirmCancelChoice::No),
            CancelConfirmationAction::Confirm
        );
    }
    assert_eq!(
        cancel_confirmation_action(special_key(KeyCode::Enter), ConfirmCancelChoice::Yes),
        CancelConfirmationAction::Confirm
    );
    assert_eq!(
        cancel_confirmation_action(special_key(KeyCode::Enter), ConfirmCancelChoice::No),
        CancelConfirmationAction::Cancel
    );
}

#[test]
fn r_opens_rename_dialog_with_existing_job_name() {
    let mut app = filtered_jobs_app();
    app.handle(AppMessage::Key(key('r')));

    match app.dialog() {
        Some(Dialog::EditJobName { id, input }) => {
            assert_eq!(id, "101512");
            assert_eq!(input.value(), "vasp-alpha");
        }
        _ => panic!("expected rename dialog"),
    }
}

#[test]
fn r_does_not_open_rename_dialog_in_resources_focus() {
    let mut app = filtered_jobs_app();
    app.set_focus(Focus::Resources);
    app.handle(AppMessage::Key(key('r')));

    assert!(app.dialog().is_none());
}

#[test]
fn r_in_details_focus_opens_rename_dialog() {
    let mut app = filtered_jobs_app();
    app.set_focus(Focus::Details);
    app.handle(AppMessage::Key(key('r')));

    assert!(matches!(app.dialog(), Some(Dialog::EditJobName { .. })));
}

#[test]
fn ctrl_t_opens_time_limit_dialog_in_resources_focus() {
    let mut app = filtered_jobs_app();
    app.set_focus(Focus::Resources);
    app.handle(AppMessage::Key(key_with_modifiers(
        't',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert!(matches!(app.dialog(), Some(Dialog::EditTimeLimit { .. })));
}

#[test]
fn ctrl_t_in_details_focus_opens_time_limit_dialog() {
    let mut app = filtered_jobs_app();
    app.set_focus(Focus::Details);
    app.handle(AppMessage::Key(key_with_modifiers(
        't',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert!(matches!(app.dialog(), Some(Dialog::EditTimeLimit { .. })));
}

#[test]
fn d_with_no_selected_job_does_not_panic_or_open_dialog() {
    let mut app = test_app(0, None);

    app.handle(AppMessage::Key(key('d')));

    assert!(app.dialog().is_none());
}

#[test]
fn ctrl_d_in_filter_dialog_does_not_open_cancel_popup() {
    let mut app = filtered_jobs_app();
    open_filter(&mut app);

    app.handle(AppMessage::Key(key_with_modifiers(
        'd',
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert!(matches!(app.dialog(), Some(Dialog::FilterJobs { .. })));
}

// ── Resources panel tests ──
