use super::*;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{fs, io, time::UNIX_EPOCH};

pub(super) fn centered_dialog_area(width: u16, lines: u16, viewport: Rect) -> Rect {
    let dialog_width = min(width, viewport.width);
    let dialog_height = min(lines, viewport.height);
    let dialog_x = viewport.x + viewport.width.saturating_sub(dialog_width) / 2;
    let dialog_y = viewport.y + viewport.height.saturating_sub(dialog_height) / 2;

    Rect::new(dialog_x, dialog_y, dialog_width, dialog_height)
}

pub(super) fn filter_popup_area(viewport: Rect) -> Rect {
    centered_dialog_area(DIALOG_WIDTH, 3, viewport)
}

pub(super) fn jobs_title(
    width: u16,
    visible_count: usize,
    total_count: usize,
    active_filter: &str,
) -> String {
    let base = if active_filter.is_empty() {
        format!(
            " Jobs ({visible_count}{}) ",
            if total_count == visible_count {
                "".to_string()
            } else {
                format!("/{total_count}")
            }
        )
    } else {
        format!(" Jobs ({visible_count}/{total_count}) filter: {active_filter} ")
    };
    truncate_with_ellipsis(&base, width.saturating_sub(2) as usize)
}

pub(super) fn truncate_with_ellipsis(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else if max_chars == 0 {
        String::new()
    } else if max_chars == 1 {
        "…".to_string()
    } else {
        let mut truncated = value.chars().take(max_chars - 1).collect::<String>();
        truncated.push('…');
        truncated
    }
}

pub(in crate::app) fn chunked_string(
    s: &str,
    first_chunk_size: usize,
    chunk_size: usize,
) -> Vec<&str> {
    let stepped_indices = s
        .char_indices()
        .map(|(i, _)| i)
        .enumerate()
        .filter(|&(i, _)| {
            if i > first_chunk_size {
                chunk_size > 0 && (i - first_chunk_size).is_multiple_of(chunk_size)
            } else {
                i == 0 || i == first_chunk_size
            }
        })
        .map(|(_, e)| e)
        .collect::<Vec<_>>();
    let windows = stepped_indices.windows(2).collect::<Vec<_>>();
    let iter = windows.iter().map(|w| &s[w[0]..w[1]]);
    let last_index = *stepped_indices.last().unwrap_or(&0);
    iter.chain(once(&s[last_index..])).collect()
}

pub(super) fn fit_text(
    s: &'_ str,
    lines: usize,
    cols: usize,
    anchor: ScrollAnchor,
    offset: usize,
    wrap: bool,
    scroll_x: usize,
) -> Text<'_> {
    let s = s.rsplit_once(['\r', '\n']).map_or(s, |(p, _)| p);
    let l = s.lines().flat_map(|line| line.split('\r'));
    let iter = match anchor {
        ScrollAnchor::Top => Either::Left(l),
        ScrollAnchor::Bottom => Either::Right(l.rev()),
    };
    let iter = iter
        .skip(offset)
        .flat_map(|line| {
            let iter = if wrap {
                Either::Left(
                    chunked_string(line, cols, cols.saturating_sub(2))
                        .into_iter()
                        .enumerate()
                        .map(|(i, chunk)| {
                            if i == 0 {
                                Line::raw(chunk.chars().take(cols).collect::<String>())
                            } else {
                                Line::default().spans(vec![
                                    Span::styled(
                                        "↪ ",
                                        Style::default().add_modifier(Modifier::DIM),
                                    ),
                                    Span::raw(
                                        chunk
                                            .chars()
                                            .take(cols.saturating_sub(2))
                                            .collect::<String>(),
                                    ),
                                ])
                            }
                        }),
                )
            } else {
                Either::Right(once(Line::raw(clip_line(line, scroll_x, cols))))
            };
            match anchor {
                ScrollAnchor::Top => Either::Left(iter),
                ScrollAnchor::Bottom => Either::Right(iter.rev()),
            }
        })
        .take(lines);

    match anchor {
        ScrollAnchor::Top => Text::from(iter.collect::<Vec<_>>()),
        ScrollAnchor::Bottom => Text::from(
            iter.collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>(),
        ),
    }
}

pub(super) fn render_workdir_text(
    entries: &[WorkdirEntry],
    message: Option<&str>,
    offset: usize,
    lines: usize,
    cols: usize,
    scroll_x: usize,
    selected: Option<usize>,
) -> Text<'static> {
    if let Some(message) = message {
        return Text::from(vec![Line::raw(clip_line(message, scroll_x, cols))]);
    }

    let name_width = entries
        .iter()
        .map(|entry| workdir_display(entry).0.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let include_year = entries
        .iter()
        .filter_map(workdir_year)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        > 1;
    let header = format!(
        "{:<name_width$} {:>8}  {:<width$}",
        "Name",
        "size",
        "mtime",
        name_width = name_width,
        width = if include_year { 16 } else { 11 }
    );
    let mut rows = vec![Line::styled(
        clip_line(&header, scroll_x, cols),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    rows.extend(
        entries
            .iter()
            .enumerate()
            .skip(offset)
            .take(lines.saturating_sub(1))
            .map(|(index, entry)| {
                let style = if Some(index) == selected {
                    Style::default().bg(Color::Green).fg(Color::Black)
                } else {
                    Style::default()
                };
                let (label, color) = workdir_display(entry);
                let line = format!(
                    "{label:<name_width$} {:>8}  {:<mtime_width$}",
                    workdir_size(entry),
                    workdir_mtime(entry, include_year),
                    name_width = name_width,
                    mtime_width = if include_year { 16 } else { 11 }
                );
                Line::from(Span::styled(
                    clip_line(&line, scroll_x, cols),
                    style.fg(color),
                ))
            })
            .collect::<Vec<_>>(),
    );
    Text::from(rows)
}

fn workdir_display(entry: &WorkdirEntry) -> (String, Color) {
    let label = match entry.kind {
        WorkdirEntryKind::Directory => format!(" {}/", entry.name),
        WorkdirEntryKind::Symlink => format!(
            " {} -> {}",
            entry.name,
            fs::read_link(&entry.path)
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        ),
        WorkdirEntryKind::File => format!(" {}", entry.name),
    };
    let color = match entry.kind {
        WorkdirEntryKind::Directory => Color::Blue,
        WorkdirEntryKind::Symlink => Color::LightBlue,
        WorkdirEntryKind::File => {
            #[cfg(unix)]
            if fs::metadata(&entry.path)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
            {
                return (label, Color::Green);
            }
            Color::Reset
        }
    };
    (label, color)
}

fn workdir_size(entry: &WorkdirEntry) -> String {
    let size = fs::metadata(&entry.path).map(|m| m.len()).unwrap_or(0);
    if size >= 1_000_000_000 {
        format!("{}G", size / 1_000_000_000)
    } else if size >= 1_000_000 {
        format!("{}M", size / 1_000_000)
    } else if size >= 1_000 {
        format!("{}K", size / 1_000)
    } else {
        format!("{}B", size)
    }
}

fn workdir_year(entry: &WorkdirEntry) -> Option<i64> {
    let Ok(duration) = fs::metadata(&entry.path)
        .and_then(|m| m.modified())
        .and_then(|t| t.duration_since(UNIX_EPOCH).map_err(io::Error::other))
    else {
        return None;
    };
    let days = (duration.as_secs() / 86_400) as i64;
    Some(civil_from_days(days).0)
}

fn workdir_mtime(entry: &WorkdirEntry, include_year: bool) -> String {
    let Ok(duration) = fs::metadata(&entry.path)
        .and_then(|m| m.modified())
        .and_then(|t| t.duration_since(UNIX_EPOCH).map_err(io::Error::other))
    else {
        return "--/-- --:--".into();
    };
    let days = (duration.as_secs() / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    let hour = (duration.as_secs() % 86_400) / 3600;
    let minute = (duration.as_secs() % 3600) / 60;
    if include_year {
        format!("{year:04}/{month:02}/{day:02} {hour:02}:{minute:02}")
    } else {
        format!("{month:02}/{day:02} {hour:02}:{minute:02}")
    }
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month, day)
}

pub(super) fn clip_line(value: &str, scroll_x: usize, width: usize) -> String {
    value.chars().skip(scroll_x).take(width).collect()
}

pub(super) fn sort_header_cell<'a>(
    first: &'a str,
    rest: &'a str,
    indicator: &'static str,
) -> Cell<'a> {
    let indicator = if indicator.is_empty() { " " } else { indicator };
    Cell::from(Line::from(vec![
        Span::styled(
            first,
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(rest, Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(indicator, Style::default().add_modifier(Modifier::BOLD)),
    ]))
}

pub(in crate::app) fn job_output_line_count(s: &str, cols: usize, wrap: bool) -> usize {
    let s = s.rsplit_once(['\r', '\n']).map_or(s, |(p, _)| p);
    let lines = s.lines().flat_map(|line| line.split('\r'));

    lines
        .map(|line| {
            if wrap {
                chunked_string(line, cols, cols.saturating_sub(2))
                    .len()
                    .max(1)
            } else {
                1
            }
        })
        .sum()
}
