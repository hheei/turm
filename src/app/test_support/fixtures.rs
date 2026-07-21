use super::*;

pub fn test_job(index: usize) -> Job {
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
        workdir: None,
        command: format!("run-job-{index}"),
    }
}

pub fn validate_time_limit(input: &Input) -> Option<String> {
    super::commands::validated_time_limit(input)
}

pub fn chunk_string(value: &str, first: usize, chunk: usize) -> Vec<&str> {
    chunked_string(value, first, chunk)
}

pub fn watched_path(job: &Job, mode: OutputPanelMode) -> Option<PathBuf> {
    watched_output_path(job, mode)
}

pub fn cancellation_action(
    key: KeyEvent,
    selected: ConfirmCancelChoice,
) -> CancelConfirmationAction {
    cancel_confirmation_action(key, selected)
}

pub fn vertical_scrollbar_thumb() -> &'static str {
    VERTICAL_SCROLLBAR_THUMB
}

pub fn horizontal_scrollbar_thumb() -> &'static str {
    OUTPUT_HORIZONTAL_SCROLLBAR_THUMB
}
