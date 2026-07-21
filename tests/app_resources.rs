mod common;
use common::*;

#[test]
fn resources_panel_renders_title_and_headers() {
    let mut app = test_app(3, Some(0));
    let buffer = draw_app(&mut app, 120, 20);
    let text = buffer_text(&buffer, 120, 20);
    assert!(text.contains("Res"));
    assert!(text.contains("Partition"));
    assert!(text.contains("Used"));
    assert!(text.contains("Avail"));
    assert!(!text.contains("Running"));
    assert!(!text.contains("Available"));
    assert!(!text.contains("Pending"));
    let header_y = app.resource_area().y.saturating_add(1);
    assert!(
        (app.resource_area().x + 2..app.resource_area().right().saturating_sub(2))
            .filter(|&x| !buffer[(x, header_y)].symbol().trim().is_empty())
            .all(|x| buffer[(x, header_y)].fg == Color::Cyan)
    );
}

#[test]
fn resources_show_group_usage_and_dim_unavailable_partitions() {
    let mut app = test_app(1, Some(0));
    app.set_resources(vec![
        turm::test_support::ResourceSnapshot {
            partition: "open".to_string(),
            total_nodes: 10,
            running_nodes: 8,
            group_used_nodes: 3,
            available_nodes: 2,
        },
        turm::test_support::ResourceSnapshot {
            partition: "full".to_string(),
            total_nodes: 12,
            running_nodes: 12,
            group_used_nodes: 4,
            available_nodes: 0,
        },
    ]);
    let buffer = draw_app(&mut app, 120, 20);
    let area = app.resource_area();
    let text = buffer_text(&buffer, 120, 20);
    let full_y = (area.y + 2..area.bottom())
        .find(|&y| row_text(&buffer, area, y).contains("full"))
        .unwrap();

    assert!(text.contains("8(3)"));
    assert!(text.contains("12(4)"));
    assert!((area.x + 2..area.right().saturating_sub(2)).all(|x| {
        buffer[(x, full_y)]
            .style()
            .add_modifier
            .contains(Modifier::DIM)
    }));
}

#[test]
fn resources_panel_shows_empty_state_when_no_data() {
    let mut app = test_app(3, Some(0));
    let buffer = draw_app(&mut app, 120, 20);
    let text = buffer_text(&buffer, 120, 20);
    assert!(text.contains("No resource"), "text was:\n{text}");
}

#[test]
fn bracket_keys_do_not_switch_focus() {
    let mut app = test_app(3, Some(0));

    // Default is Jobs
    assert_eq!(app.focus(), Focus::Jobs);

    app.handle(AppMessage::Key(key('[')));
    app.handle(AppMessage::Key(key(']')));
    assert_eq!(app.focus(), Focus::Jobs);
}

#[test]
fn resources_scrollbar_hidden_when_rows_fit() {
    let mut app = test_app(3, Some(0));
    let buffer = draw_app(&mut app, 120, 20);
    let symbols = jobs_area_symbols(&buffer, app.resource_area());
    // With empty resources (1 placeholder row), row fits in 8-height panel
    assert!(!symbols.iter().any(|s| s == VERTICAL_SCROLLBAR_THUMB));
}

#[test]
fn help_line_shows_resources_mode_when_focused() {
    let mut app = test_app(3, Some(0));
    app.set_focus(Focus::Resources);
    let text = buffer_text(&draw_app(&mut app, 120, 20), 120, 20);
    assert!(!text.contains("mode:"));
    assert!(text.contains("⇥ toggle"));
    assert!(!text.contains("focus"));
    assert!(!text.contains("↑/↓: move"));
    assert!(!text.contains("j/k"));
}

#[test]
fn resources_panel_uses_top_left_area() {
    let mut app = test_app(3, Some(0));
    let _ = draw_app(&mut app, 100, 20);
    // Resources and JOB/Details share the top row.
    assert_eq!(app.resource_area().x, 0);
    assert_eq!(app.resource_area().y, 0);
    assert_eq!(app.resource_area().width, 35);
    assert_eq!(app.job_list_area().x, 34);
    assert_eq!(app.job_list_area().width, 66);
    assert_eq!(app.job_list_area().y, app.resource_area().y);
    assert!(app.job_details_area().y > app.resource_area().y);
    assert!(app.job_details_area().bottom() <= app.resource_area().bottom());
}
