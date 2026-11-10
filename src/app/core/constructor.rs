use std::cell::RefCell;
impl App {
    pub fn new(
        input_receiver: Receiver<std::io::Result<Event>>,
        slurm_refresh_rate: u64,
        file_refresh_rate: u64,
        squeue_args: Vec<String>,
    ) -> App {
        Self::with_history_refresh(
            input_receiver,
            slurm_refresh_rate,
            60,
            file_refresh_rate,
            squeue_args,
        )
    }

    pub fn with_history_refresh(
        input_receiver: Receiver<std::io::Result<Event>>,
        slurm_refresh_rate: u64,
        history_refresh_rate: u64,
        file_refresh_rate: u64,
        squeue_args: Vec<String>,
    ) -> App {
        let (sender, receiver) = unbounded();
        let queries = crate::slurm::jobs::JobQueries::new(
            &squeue_args,
            std::env::var("USER").ok().as_deref(),
        );
        let job_snapshots = JobSnapshots::new(&queries);
        let slurm = SlurmHandle::start(
            sender.clone(),
            Duration::from_secs(slurm_refresh_rate),
            Duration::from_secs(history_refresh_rate),
            queries,
            crate::slurm::SlurmClient::default(),
        );
        Self {
            focus: Focus::Jobs,
            dialog: None,
            jobs: Vec::new(),
            active_filter: String::new(),
            visible_job_cache: RefCell::new(None),
            jobs_generation: 0,
            slurm,
            job_snapshots,
            command_failures: Default::default(),
            job_list_state: TableState::new(),
            job_sort_field: JobSortField::Time,
            job_sort_direction: SortDirection::Desc,
            job_output: Ok("".to_string()),
            job_output_anchor: ScrollAnchor::Bottom,
            job_output_offset: 0,
            output_scroll_x: 0,
            job_output_wrap: false,
            workdir_path: None,
            workdir_entries: Vec::new(),
            workdir_error: None,
            workdir_selected: None,
            workdir_offset: 0,
            job_output_watcher: FileWatcherHandle::new(
                sender.clone(),
                Duration::from_secs(file_refresh_rate),
            ),
            // sender,
            // sender,
            receiver,
            input_receiver,
            output_panel_mode: OutputPanelMode::default(),
            output_can_expand: false,
            details_visible: true,
            job_list_height: 0,
            job_list_area: Rect::default(),
            job_details_area: Rect::default(),
            job_output_area: Rect::default(),
            pending_input_event: None,
            pending_clipboard_copy: None,
            clipboard_notice_until: None,
            pending_exit: None,
            mouse_selection: None,
            details_selection_rows: Vec::new(),
            screen_buffer: None,
            resource_table_state: TableState::new(),
            resource_list_height: 0,
            resource_area: Rect::default(),
            resources: Vec::new(),
        }
    }
}
use super::*;
