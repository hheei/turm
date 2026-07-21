use super::*;

impl AppDriver {
    pub fn new(job_count: usize, selected: Option<usize>) -> Self {
        let (app_sender, receiver) = unbounded();
        let (_input_sender, input_receiver) = unbounded();
        let jobs = (0..job_count).map(test_job).collect();

        Self {
            app: App {
                focus: Focus::Jobs,
                dialog: None,
                jobs,
                active_filter: String::new(),
                job_list_state: TableState::new().with_selected(selected),
                job_sort_field: JobSortField::Time,
                job_sort_direction: SortDirection::Asc,
                job_output: Ok(String::new()),
                job_output_anchor: ScrollAnchor::Bottom,
                job_output_offset: 0,
                output_scroll_x: 0,
                job_output_wrap: false,
                workdir_path: None,
                workdir_entries: Vec::new(),
                workdir_error: None,
                workdir_selected: None,
                workdir_offset: 0,
                _job_watcher: JobWatcherHandle {},
                _resource_watcher: ResourceWatcherHandle {},
                job_output_watcher: FileWatcherHandle::new(app_sender, Duration::from_secs(60)),
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
            },
        }
    }

    pub fn with_jobs(jobs: Vec<Job>, selected: Option<usize>) -> Self {
        let mut driver = Self::new(0, selected);
        driver.app.jobs = jobs;
        driver
    }

    pub fn render(&mut self, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
        terminal
            .draw(|frame| self.app.ui(frame))
            .expect("test terminal should draw");
        terminal.backend().buffer().clone()
    }

    pub fn handle(&mut self, message: AppMessage) {
        self.app.handle(message);
    }

    pub fn snapshot(&self) -> AppSnapshot {
        AppSnapshot {
            focus: self.focus(),
            has_dialog: self.dialog().is_some(),
            selected_job_index: self.selected_job_index(),
            active_filter: self.active_filter().to_string(),
            sort_field: self.sort_field(),
            sort_direction: self.sort_direction(),
            output_mode: self.output_mode(),
            details_visible: self.details_visible(),
            output_anchor: self.output_anchor(),
            output_offset: self.output_offset(),
            output_scroll_x: self.output_scroll_x(),
            output_wrap: self.output_wrap(),
            workdir_selected: self.workdir_selected(),
            workdir_offset: self.workdir_offset(),
            pending_clipboard_copy: self.pending_clipboard_copy().map(str::to_string),
            job_list_area: self.job_list_area(),
            job_details_area: self.job_details_area(),
            job_output_area: self.job_output_area(),
            resource_area: self.resource_area(),
        }
    }

    pub fn handle_input_event(&mut self, event: Event) -> (bool, bool) {
        self.app.handle_input_event(event)
    }

    pub fn apply_job_filter(&mut self, filter: &str) {
        self.app.apply_job_filter(filter);
    }

    pub fn sort_jobs(&mut self) {
        self.app.sort_jobs();
    }

    pub fn scroll_jobs_half_page_up(&mut self) {
        self.app.scroll_jobs_half_page_up();
    }

    pub fn focus(&self) -> Focus {
        self.app.focus
    }

    pub fn set_focus(&mut self, focus: Focus) {
        self.app.focus = focus;
    }

    pub fn dialog(&self) -> Option<&Dialog> {
        self.app.dialog.as_ref()
    }

    pub fn set_dialog(&mut self, dialog: Option<Dialog>) {
        self.app.dialog = dialog;
    }

    pub fn jobs(&self) -> &[Job] {
        &self.app.jobs
    }

    pub fn jobs_mut(&mut self) -> &mut [Job] {
        &mut self.app.jobs
    }

    pub fn active_filter(&self) -> &str {
        &self.app.active_filter
    }

    pub fn set_active_filter(&mut self, filter: impl Into<String>) {
        self.app.active_filter = filter.into();
    }

    pub fn selected_job_index(&self) -> Option<usize> {
        self.app.job_list_state.selected()
    }

    pub fn job_list_offset(&self) -> usize {
        self.app.job_list_state.offset()
    }

    pub fn selected_job_id(&self) -> Option<String> {
        self.app.selected_job_id()
    }

    pub fn visible_job_ids(&self) -> Vec<String> {
        self.app
            .visible_job_indices()
            .into_iter()
            .map(|index| self.app.jobs[index].id())
            .collect()
    }

    pub fn job_ids(&self) -> Vec<String> {
        self.app.jobs.iter().map(Job::id).collect()
    }

    pub fn sort_field(&self) -> TestJobSortField {
        match self.app.job_sort_field {
            JobSortField::State => TestJobSortField::State,
            JobSortField::Partition => TestJobSortField::Partition,
            JobSortField::Id => TestJobSortField::Id,
            JobSortField::Name => TestJobSortField::Name,
            JobSortField::User => TestJobSortField::User,
            JobSortField::Time => TestJobSortField::Time,
        }
    }

    pub fn sort_direction(&self) -> TestSortDirection {
        match self.app.job_sort_direction {
            SortDirection::Asc => TestSortDirection::Asc,
            SortDirection::Desc => TestSortDirection::Desc,
        }
    }

    pub fn set_sort(&mut self, field: TestJobSortField, direction: TestSortDirection) {
        self.app.job_sort_field = match field {
            TestJobSortField::State => JobSortField::State,
            TestJobSortField::Partition => JobSortField::Partition,
            TestJobSortField::Id => JobSortField::Id,
            TestJobSortField::Name => JobSortField::Name,
            TestJobSortField::User => JobSortField::User,
            TestJobSortField::Time => JobSortField::Time,
        };
        self.app.job_sort_direction = match direction {
            TestSortDirection::Asc => SortDirection::Asc,
            TestSortDirection::Desc => SortDirection::Desc,
        };
    }

    pub fn set_job_output(&mut self, output: impl Into<String>) {
        self.app.job_output = Ok(output.into());
    }

    pub fn output_anchor(&self) -> ScrollAnchor {
        self.app.job_output_anchor
    }

    pub fn set_output_anchor(&mut self, anchor: ScrollAnchor) {
        self.app.job_output_anchor = anchor;
    }

    pub fn output_offset(&self) -> u16 {
        self.app.job_output_offset
    }

    pub fn set_output_offset(&mut self, offset: u16) {
        self.app.job_output_offset = offset;
    }

    pub fn output_scroll_x(&self) -> u16 {
        self.app.output_scroll_x
    }

    pub fn output_wrap(&self) -> bool {
        self.app.job_output_wrap
    }

    pub fn set_output_wrap(&mut self, wrap: bool) {
        self.app.job_output_wrap = wrap;
    }

    pub fn output_mode(&self) -> OutputPanelMode {
        self.app.output_panel_mode
    }

    pub fn set_output_mode(&mut self, mode: OutputPanelMode) {
        self.app.output_panel_mode = mode;
    }

    pub fn details_visible(&self) -> bool {
        self.app.details_visible
    }

    pub fn layout(&self) -> LayoutSnapshot {
        let OutputLayout {
            viewport,
            show_vertical,
            show_horizontal,
        } = self.app.output_layout();
        LayoutSnapshot {
            viewport,
            show_vertical,
            show_horizontal,
        }
    }

    pub fn max_output_scroll_x(&self) -> u16 {
        self.app.max_output_scroll_x()
    }

    pub fn max_output_offset(&self) -> u16 {
        self.app.max_job_output_offset()
    }

    pub fn job_list_rows_area(&self) -> Rect {
        self.app.job_list_rows_area()
    }

    pub fn job_list_height(&self) -> u16 {
        self.app.job_list_height
    }

    pub fn job_list_area(&self) -> Rect {
        self.app.job_list_area
    }

    pub fn job_details_area(&self) -> Rect {
        self.app.job_details_area
    }

    pub fn job_output_area(&self) -> Rect {
        self.app.job_output_area
    }

    pub fn resource_area(&self) -> Rect {
        self.app.resource_area
    }

    pub fn set_resources(&mut self, resources: Vec<ResourceSnapshot>) {
        self.app.resources = resources
            .into_iter()
            .map(PartitionResources::from)
            .collect();
    }

    pub fn set_workdir_entries(&mut self, entries: Vec<TestWorkdirEntry>) {
        self.app.workdir_entries = entries
            .into_iter()
            .map(|entry| WorkdirEntry {
                name: entry.name,
                path: entry.path,
                kind: match entry.kind {
                    TestWorkdirEntryKind::Directory => WorkdirEntryKind::Directory,
                    TestWorkdirEntryKind::File => WorkdirEntryKind::File,
                    TestWorkdirEntryKind::Symlink => WorkdirEntryKind::Symlink,
                },
            })
            .collect();
    }

    pub fn workdir_entry_count(&self) -> usize {
        self.app.workdir_entries.len()
    }

    pub fn workdir_selected(&self) -> Option<usize> {
        self.app.workdir_selected
    }

    pub fn set_workdir_selected(&mut self, selected: Option<usize>) {
        self.app.workdir_selected = selected;
    }

    pub fn workdir_offset(&self) -> usize {
        self.app.workdir_offset
    }

    pub fn set_workdir_offset(&mut self, offset: usize) {
        self.app.workdir_offset = offset;
    }

    pub fn pending_clipboard_copy(&self) -> Option<&str> {
        self.app.pending_clipboard_copy.as_deref()
    }

    pub fn pending_exit(&self) -> Option<&AppExit> {
        self.app.pending_exit.as_ref()
    }

    pub fn derive_workdir_path(job: &Job) -> Option<PathBuf> {
        App::derive_workdir_path(job)
    }
}
