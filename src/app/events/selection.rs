use super::*;

impl App {
    fn selection_target(&self, column: u16, row: u16) -> Option<(SelectionArea, Rect)> {
        let panels = [
            (SelectionArea::Resources, self.resource_area),
            (SelectionArea::Details, self.job_details_area),
            (SelectionArea::Jobs, self.job_list_area),
            (SelectionArea::Output, self.job_output_area),
        ];
        panels.into_iter().find_map(|(area, panel)| {
            let bounds = match area {
                SelectionArea::Resources | SelectionArea::Jobs => Rect::new(
                    panel.x.saturating_add(2),
                    panel.y.saturating_add(1),
                    panel.width.saturating_sub(4),
                    panel.height.saturating_sub(2),
                ),
                SelectionArea::Details => Rect::new(
                    panel.x.saturating_add(2),
                    panel.y.saturating_add(1),
                    panel.width.saturating_sub(4),
                    panel.height.saturating_sub(2),
                ),
                SelectionArea::Output => self.output_layout().viewport,
            };
            rect_contains(bounds, column, row).then_some((area, bounds))
        })
    }

    pub(in crate::app) fn begin_mouse_selection(&mut self, column: u16, row: u16) {
        self.mouse_selection = self.selection_target(column, row).map(|(area, bounds)| {
            let point = Position { x: column, y: row };
            MouseSelection {
                area,
                bounds,
                start: point,
                end: point,
                dragged: false,
            }
        });
    }

    pub(in crate::app) fn update_mouse_selection(&mut self, column: u16, row: u16) {
        let Some(selection) = &mut self.mouse_selection else {
            return;
        };
        let end = Position {
            x: column.clamp(
                selection.bounds.x,
                selection
                    .bounds
                    .x
                    .saturating_add(selection.bounds.width.saturating_sub(1)),
            ),
            y: row.clamp(
                selection.bounds.y,
                selection
                    .bounds
                    .y
                    .saturating_add(selection.bounds.height.saturating_sub(1)),
            ),
        };
        selection.dragged |= end != selection.start;
        selection.end = end;
    }

    pub(in crate::app) fn mouse_selection_row_bounds(
        &self,
        selection: MouseSelection,
        row: u16,
    ) -> Option<(u16, u16)> {
        if selection.area == SelectionArea::Details {
            return self.details_selection_row_bounds(selection, row);
        }
        let (left, right) = selection.row_bounds(row)?;
        let details = self.job_details_area;
        if selection.area == SelectionArea::Jobs
            && (details.y..details.bottom()).contains(&row)
            && right >= details.x
            && left < details.right()
        {
            return (left < details.x).then_some((left, details.x.saturating_sub(1)));
        }
        Some((left, right))
    }

    pub(in crate::app) fn mouse_selection_y_bounds(&self, selection: MouseSelection) -> (u16, u16) {
        if selection.area != SelectionArea::Details {
            return (
                selection.start.y.min(selection.end.y),
                selection.start.y.max(selection.end.y),
            );
        }
        let groups = self
            .details_selection_rows
            .iter()
            .filter(|row| row.y == selection.start.y || row.y == selection.end.y)
            .map(|row| row.group)
            .collect::<Vec<_>>();
        let Some((&first, &last)) = groups.iter().min().zip(groups.iter().max()) else {
            return (selection.start.y, selection.end.y);
        };
        let mut rows = self
            .details_selection_rows
            .iter()
            .filter(|row| (first..=last).contains(&row.group))
            .map(|row| row.y);
        let Some(top) = rows.next() else {
            return (selection.start.y, selection.end.y);
        };
        rows.fold((top, top), |(min, max), y| (min.min(y), max.max(y)))
    }

    fn details_selection_row_bounds(
        &self,
        selection: MouseSelection,
        row: u16,
    ) -> Option<(u16, u16)> {
        let start_row = self
            .details_selection_rows
            .iter()
            .find(|item| item.y == selection.start.y)?;
        let end_row = self
            .details_selection_rows
            .iter()
            .find(|item| item.y == selection.end.y)?;
        let current = self
            .details_selection_rows
            .iter()
            .find(|item| item.y == row)?;
        let endpoint = |item: &DetailsSelectionRow, x: u16| {
            (item.group * 2 + usize::from(x >= item.value_x), item.y, x)
        };
        let mut start = endpoint(start_row, selection.start.x);
        let mut end = endpoint(end_row, selection.end.x);
        if start > end {
            std::mem::swap(&mut start, &mut end);
        }

        let key_order = current.group * 2;
        let value_order = key_order + 1;
        let key_selected = (start.0..=end.0).contains(&key_order);
        let value_selected = (start.0..=end.0).contains(&value_order);
        if !key_selected && !value_selected {
            return None;
        }

        let mut left = if key_selected {
            current.left
        } else {
            current.value_x
        };
        let mut right = if value_selected {
            current.right
        } else {
            current.value_x.saturating_sub(1)
        };
        if start.0 == value_order {
            if row < start.1 {
                return key_selected.then_some((left, current.value_x.saturating_sub(1)));
            }
            if row == start.1 {
                left = start.2;
            }
        }
        if end.0 == value_order {
            if row > end.1 {
                return key_selected.then_some((left, current.value_x.saturating_sub(1)));
            }
            if row == end.1 {
                right = end.2;
            }
        }
        (left <= right).then_some((left, right))
    }

    pub(in crate::app) fn selected_mouse_text(&self) -> Option<String> {
        let selection = self.mouse_selection?;
        let buffer = self.screen_buffer.as_ref()?;
        let (top, bottom) = self.mouse_selection_y_bounds(selection);
        let text = (top..=bottom)
            .filter_map(|y| {
                let (left, right) = self.mouse_selection_row_bounds(selection, y)?;
                Some(
                    (left..=right)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                        .trim_end()
                        .to_string(),
                )
            })
            .collect::<Vec<_>>();
        Some(text.join("\n"))
    }

    pub(in crate::app) fn clear_mouse_selection(&mut self) {
        self.mouse_selection = None;
    }

    pub(in crate::app) fn selected_job(&self) -> Option<&Job> {
        let visible_job_indices = self.visible_job_indices();
        self.job_list_state
            .selected()
            .and_then(|index| visible_job_indices.get(index).copied())
            .and_then(|index| self.jobs.get(index))
    }

    pub(in crate::app) fn selected_job_id(&self) -> Option<String> {
        self.selected_job().map(Job::id)
    }

    pub(in crate::app) fn flush_pending_clipboard_copy(&mut self) -> io::Result<()> {
        if let Some(value) = self.pending_clipboard_copy.take() {
            write_osc52_clipboard(&value)?;
        }

        Ok(())
    }

    pub(in crate::app) fn copy_job_output_directory_dialog(&self) -> Option<Dialog> {
        let directory = self
            .selected_job()
            .and_then(|job| output_directory_for_mode(job, self.output_panel_mode))?;
        let (dir_url, dir_name) = copy_job_output_directory_value(&directory)?;

        Some(Dialog::CopyJobOutputDirectory { dir_url, dir_name })
    }

    pub(in crate::app) fn cancel_confirmation_dialog(&self) -> Option<Dialog> {
        let job = self.selected_job()?;
        Some(Dialog::ConfirmCancelJob {
            id: job.id(),
            name: job.name.clone(),
            details: selected_job_cancel_details(job),
            signal: None,
            selected: ConfirmCancelChoice::No,
        })
    }
}
