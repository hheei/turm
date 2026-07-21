mod common;
use common::*;

#[test]
fn workdir_path_derivation_prefers_explicit_then_stdout_then_stderr_then_command() {
    let mut job = test_job(0);
    job.workdir = Some(PathBuf::from("/explicit/workdir"));
    job.stdout = Some(PathBuf::from("/stdout/dir/stdout.log"));
    job.stderr = Some(PathBuf::from("/stderr/dir/stderr.log"));
    job.command = "/command/dir/run.sh".to_string();
    assert_eq!(
        AppDriver::derive_workdir_path(&job),
        Some(PathBuf::from("/explicit/workdir"))
    );

    job.workdir = None;
    assert_eq!(
        AppDriver::derive_workdir_path(&job),
        Some(PathBuf::from("/stdout/dir"))
    );

    job.stdout = None;
    assert_eq!(
        AppDriver::derive_workdir_path(&job),
        Some(PathBuf::from("/stderr/dir"))
    );

    job.stderr = None;
    assert_eq!(
        AppDriver::derive_workdir_path(&job),
        Some(PathBuf::from("/command/dir"))
    );
}

#[test]
fn workdir_mode_reports_missing_path() {
    let mut app = test_app(1, Some(0));
    app.set_output_mode(OutputPanelMode::Collapsed);
    let _ = draw_app(&mut app, 120, 40);
    app.handle(AppMessage::Key(key('o')));
    let text = buffer_text(&draw_app(&mut app, 100, 29), 100, 29);

    assert!(text.contains("No workdir available"));
}

#[test]
fn workdir_listing_is_shallow_sorted_and_classified() {
    let temp_root = std::env::temp_dir().join(format!("turm-workdir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(temp_root.join("calc")).unwrap();
    std::fs::create_dir_all(temp_root.join("logs")).unwrap();
    std::fs::create_dir_all(temp_root.join("nested").join("deep")).unwrap();
    std::fs::write(temp_root.join("INCAR"), "incar").unwrap();
    std::fs::write(temp_root.join("POSCAR"), "poscar").unwrap();
    std::fs::write(temp_root.join("stdout.log"), "stdout").unwrap();
    std::fs::write(
        temp_root.join("nested").join("deep").join("hidden.txt"),
        "nested",
    )
    .unwrap();

    let mut app = test_app(1, Some(0));
    app.jobs_mut()[0].workdir = Some(temp_root.clone());
    app.set_output_mode(OutputPanelMode::Collapsed);
    let _ = draw_app(&mut app, 120, 40);
    app.handle(AppMessage::Key(key('o')));

    let text = buffer_text(&draw_app(&mut app, 120, 50), 120, 50);

    let calc_pos = text.find(" calc/").unwrap();
    let logs_pos = text.find(" logs/").unwrap();
    let nested_pos = text.find(" nested/").unwrap();
    let incar_pos = text.find(" INCAR").unwrap();
    let poscar_pos = text.find(" POSCAR").unwrap();
    let stdout_pos = text.find(" stdout.log").unwrap();
    assert!(calc_pos < incar_pos);
    assert!(logs_pos < incar_pos);
    assert!(nested_pos < incar_pos);
    assert!(incar_pos < poscar_pos);
    assert!(poscar_pos < stdout_pos);
    assert!(!text.contains("hidden.txt"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn workdir_mode_handles_unreadable_directory_without_panic() {
    let temp_root = std::env::temp_dir().join(format!("turm-missing-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_root);

    let mut app = test_app(1, Some(0));
    app.jobs_mut()[0].workdir = Some(temp_root.clone());
    app.set_output_mode(OutputPanelMode::Collapsed);
    let _ = draw_app(&mut app, 120, 40);
    app.handle(AppMessage::Key(key('o')));

    let text = buffer_text(&draw_app(&mut app, 120, 29), 120, 29);

    assert!(text.contains("Unable to read workdir:"));
    assert!(text.contains(temp_root.to_string_lossy().as_ref()));
}
