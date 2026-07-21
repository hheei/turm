use std::path::Path;
use turm::test_support::{
    ResourceSnapshot, fetch_resources_from, parse_sinfo_json, parse_sinfo_text,
    sort_resource_snapshots,
};

#[test]
fn counts_allocated_as_running() {
    let json = serde_json::json!({
        "nodes": [
            {"partition": "debug", "state": "allocated"},
            {"partition": "debug", "state": "allocated"}
        ]
    });
    let resources = parse_sinfo_json(&json);
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].running_nodes, 2);
    assert_eq!(resources[0].available_nodes, 0);
}

#[test]
fn counts_mixed_as_running() {
    let json = serde_json::json!({
        "nodes": [
            {"partition": "gpu", "state": "MIXED"},
            {"partition": "gpu", "state": "mix"},
            {"partition": "gpu", "state": "ALLOCATED"}
        ]
    });
    let resources = parse_sinfo_json(&json);
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].partition, "gpu");
    assert_eq!(resources[0].running_nodes, 3);
    assert_eq!(resources[0].available_nodes, 0);
}

#[test]
fn counts_idle_as_available() {
    let json = serde_json::json!({
        "nodes": [
            {"partition": "debug", "state": "idle"},
            {"partition": "debug", "state": "IDLE"},
            {"partition": "debug", "state": "allocated"}
        ]
    });
    let resources = parse_sinfo_json(&json);
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].running_nodes, 1);
    assert_eq!(resources[0].available_nodes, 2);
}

#[test]
fn excludes_unavailable_states() {
    let json = serde_json::json!({
        "nodes": [
            {"partition": "debug", "state": "down"},
            {"partition": "debug", "state": "drain"},
            {"partition": "debug", "state": "draining"},
            {"partition": "debug", "state": "drained"},
            {"partition": "debug", "state": "fail"},
            {"partition": "debug", "state": "failing"},
            {"partition": "debug", "state": "reserved"},
            {"partition": "debug", "state": "unknown"},
            {"partition": "debug", "state": "idle"},
            {"partition": "debug", "state": "allocated"}
        ]
    });
    let resources = parse_sinfo_json(&json);
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].running_nodes, 1);
    assert_eq!(resources[0].available_nodes, 1);
}

#[test]
fn groups_and_sorts_by_partition() {
    let json = serde_json::json!({
        "nodes": [
            {"partition": "gpu", "state": "allocated"},
            {"partition": "debug", "state": "idle"},
            {"partition": "cpu", "state": "mixed"},
            {"partition": "gpu", "state": "idle"},
            {"partition": "debug", "state": "allocated"},
            {"partition": "cpu", "state": "idle"}
        ]
    });
    let resources = parse_sinfo_json(&json);
    assert_eq!(resources.len(), 3);
    assert_eq!(resources[0].partition, "cpu");
    assert_eq!(resources[0].running_nodes, 1);
    assert_eq!(resources[0].available_nodes, 1);
    assert_eq!(resources[1].partition, "debug");
    assert_eq!(resources[1].running_nodes, 1);
    assert_eq!(resources[1].available_nodes, 1);
    assert_eq!(resources[2].partition, "gpu");
    assert_eq!(resources[2].running_nodes, 1);
    assert_eq!(resources[2].available_nodes, 1);
}

#[test]
fn missing_nodes_field_returns_empty() {
    let resources = parse_sinfo_json(&serde_json::json!({"partitions": []}));
    assert!(resources.is_empty());
}

#[test]
fn missing_partition_or_state_skips_node() {
    let json = serde_json::json!({
        "nodes": [
            {"state": "idle"},
            {"partition": "debug"},
            {"partition": "debug", "state": "allocated"}
        ]
    });
    let resources = parse_sinfo_json(&json);
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].running_nodes, 1);
    assert_eq!(resources[0].available_nodes, 0);
}

#[test]
fn plain_parser_counts_with_node_count_field() {
    let resources = parse_sinfo_text("debug alloc 3\ndebug idle 2\n");
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].running_nodes, 3);
    assert_eq!(resources[0].available_nodes, 2);
}

#[test]
fn plain_parser_counts_mix_as_running() {
    let resources = parse_sinfo_text("gpu mix 2\ngpu alloc 1\ngpu idle 4\n");
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].running_nodes, 3);
    assert_eq!(resources[0].available_nodes, 4);
}

#[test]
fn plain_parser_defaults_to_1_when_count_missing() {
    let resources = parse_sinfo_text("debug alloc\ndebug idle\n");
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].running_nodes, 1);
    assert_eq!(resources[0].available_nodes, 1);
}

#[test]
fn plain_parser_ignores_unavailable_states() {
    let resources = parse_sinfo_text("debug down 5\ndebug drain 3\ndebug alloc 2\ndebug idle 1\n");
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].running_nodes, 2);
    assert_eq!(resources[0].available_nodes, 1);
}

#[test]
fn plain_parser_groups_by_partition() {
    let resources = parse_sinfo_text("gpu alloc 1\ndebug idle 2\ncpu mix 3\ncpu idle 4\n");
    assert_eq!(resources.len(), 3);
    assert_eq!(resources[0].partition, "cpu");
    assert_eq!(resources[0].running_nodes, 3);
    assert_eq!(resources[0].available_nodes, 4);
}

#[test]
fn sort_resources_orders_by_available_desc_then_partition() {
    let resources = sort_resource_snapshots(vec![
        ResourceSnapshot {
            partition: "A".into(),
            running_nodes: 1,
            available_nodes: 5,
        },
        ResourceSnapshot {
            partition: "B".into(),
            running_nodes: 2,
            available_nodes: 10,
        },
        ResourceSnapshot {
            partition: "C".into(),
            running_nodes: 3,
            available_nodes: 0,
        },
    ]);
    assert_eq!(resources[0].partition, "B");
    assert_eq!(resources[0].available_nodes, 10);
    assert_eq!(resources[1].partition, "A");
    assert_eq!(resources[1].available_nodes, 5);
    assert_eq!(resources[2].partition, "C");
    assert_eq!(resources[2].available_nodes, 0);
}

#[test]
fn mock_sinfo_integration_returns_resources() {
    let sinfo = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/mock-slurm/bin/sinfo");
    let resources = fetch_resources_from(&sinfo).expect("mock sinfo should succeed");
    assert!(!resources.is_empty(), "mock resources should not be empty");
}
