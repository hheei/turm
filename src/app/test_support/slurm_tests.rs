use super::*;

fn enter() -> AppMessage {
    AppMessage::Key(KeyEvent::new(
        KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    ))
}

#[test]
fn dialogs_submit_owned_commands_and_return_before_completion() {
    let requests = [
        (
            Dialog::EditJobName {
                id: "1000".into(),
                original_name: "old".into(),
                input: Input::new("new name".into()),
            },
            SlurmCommand::Rename {
                id: "1000".into(),
                value: "new name".into(),
            },
        ),
        (
            Dialog::EditTimeLimit {
                id: "1000".into(),
                maximum: "1:00:00".into(),
                input: Input::new("00:30:00".into()),
            },
            SlurmCommand::SetTimeLimit {
                id: "1000".into(),
                value: "00:30:00".into(),
            },
        ),
        (
            Dialog::ConfirmCancelJob {
                id: "1000".into(),
                name: "job".into(),
                details: Vec::new(),
                signal: Some("TERM".into()),
                selected: ConfirmCancelChoice::Yes,
            },
            SlurmCommand::Cancel {
                id: "1000".into(),
                signal: Some("TERM".into()),
            },
        ),
    ];
    for (dialog, expected) in requests {
        let mut driver = AppDriver::new(2, Some(0));
        driver.set_output_mode(OutputPanelMode::Collapsed);
        driver.set_dialog(Some(dialog));
        driver.handle(enter());
        assert!(driver.dialog().is_none());
        assert_eq!(driver.pending_slurm_request_count(), 1);
        assert_eq!(driver.slurm_requests.try_recv().unwrap(), expected);
        // No completion has arrived, but another UI action still works.
        driver.handle(AppMessage::Key(KeyEvent::new(
            KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        )));
        assert_eq!(driver.selected_job_index(), Some(1));
    }
}

#[test]
fn delayed_command_errors_queue_without_destroying_the_current_dialog() {
    let mut driver = AppDriver::new(1, Some(0));
    driver.set_dialog(Some(Dialog::FilterJobs {
        input: Input::new("name:job".into()),
    }));
    for command in ["scancel", "scontrol"] {
        driver.handle(AppMessage::Slurm(SlurmEvent::CommandFinished(Err(
            CommandFailure {
                command: command.into(),
                output: "permission denied".into(),
            },
        ))));
    }
    assert!(
        matches!(driver.dialog(), Some(Dialog::FilterJobs { input }) if input.value() == "name:job")
    );
    driver.handle(enter());
    assert_eq!(driver.active_filter(), "name:job");
    assert!(
        matches!(driver.dialog(), Some(Dialog::CommandError { command, .. }) if command == "scancel")
    );
    driver.handle(enter());
    assert!(
        matches!(driver.dialog(), Some(Dialog::CommandError { command, .. }) if command == "scontrol")
    );
    driver.handle(enter());
    assert!(driver.dialog().is_none());
}

#[test]
fn independent_snapshots_preserve_selection_and_failed_queries_keep_last_data() {
    let mut driver = AppDriver::new(0, None);
    let t = Instant::now();
    driver.handle(AppMessage::Slurm(SlurmEvent::Jobs(Ok(
        crate::slurm::JobSnapshot {
            source: crate::slurm::JobSource::Active,
            started_at: t,
            jobs: vec![test_job(0), test_job(1)],
        },
    ))));
    driver.handle(AppMessage::Key(KeyEvent::new(
        KeyCode::Down,
        crossterm::event::KeyModifiers::NONE,
    )));
    let selected = driver.app.selected_job_id();
    let mut completed = test_job(2);
    completed.state = "COMPLETED".into();
    completed.state_compact = "CD".into();
    driver.handle(AppMessage::Slurm(SlurmEvent::Jobs(Ok(
        crate::slurm::JobSnapshot {
            source: crate::slurm::JobSource::History,
            started_at: t,
            jobs: vec![completed],
        },
    ))));
    assert_eq!(driver.jobs().len(), 3);
    assert_eq!(driver.app.selected_job_id(), selected);
    driver.handle(AppMessage::Slurm(SlurmEvent::Jobs(Err(
        "unavailable".into()
    ))));
    assert_eq!(driver.jobs().len(), 3);
    assert_eq!(driver.app.selected_job_id(), selected);
}
