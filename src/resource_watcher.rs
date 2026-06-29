use std::collections::BTreeMap;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crossbeam::channel::Sender;

use crate::app::AppMessage;
use crate::app::PartitionResources;

struct ResourceWatcher {
    app: Sender<AppMessage>,
    interval: Duration,
}

pub struct ResourceWatcherHandle {}

impl ResourceWatcher {
    fn new(app: Sender<AppMessage>, interval: Duration) -> Self {
        Self { app, interval }
    }

    fn run(&mut self) {
        loop {
            let result = fetch_resources();
            match result {
                Ok(resources) => {
                    let _ = self.app.send(AppMessage::ResourcesUpdated(resources));
                }
                Err(err) => {
                    let _ = self
                        .app
                        .send(AppMessage::ResourceWatcherError(err.to_string()));
                }
            }
            thread::sleep(self.interval);
        }
    }
}

impl ResourceWatcherHandle {
    pub fn new(app: Sender<AppMessage>, interval: Duration) -> Self {
        let mut watcher = ResourceWatcher::new(app, interval);
        thread::spawn(move || watcher.run());
        Self {}
    }
}

// ── sinfo fetching ──

/// Runs `sinfo` and returns parsed partition resources.
/// Tries `--json` first (Slurm 23.02+), falls back to plain format.
pub(crate) fn fetch_resources() -> Result<Vec<PartitionResources>, Box<dyn std::error::Error>> {
    // Try --json first (newer Slurm)
    if let Ok(output) = Command::new("sinfo").arg("--json").output() {
        if output.status.success() {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                return Ok(sort_resources(parse_sinfo_resources(&value)));
            }
        }
    }
    // Fallback: plain sinfo output (Slurm 20.11 and older)
    let output = Command::new("sinfo")
        .args(["-o", "%P %t %D", "--noheader"])
        .output()?;
    if !output.status.success() {
        return Err("sinfo command failed".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(sort_resources(parse_sinfo_plain(&text)))
}

/// Sort resources: Available descending, then partition name ascending.
fn sort_resources(mut resources: Vec<PartitionResources>) -> Vec<PartitionResources> {
    resources.sort_by(|a, b| {
        b.available_nodes
            .cmp(&a.available_nodes)
            .then(a.partition.cmp(&b.partition))
    });
    resources
}

/// Parse plain `sinfo -o '%P %t %D' --noheader` output.
/// Format: "partition state count" per line, e.g. "debug alloc 3".
pub(crate) fn parse_sinfo_plain(text: &str) -> Vec<PartitionResources> {
    let mut partition_map: BTreeMap<String, PartitionResources> = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(partition) = parts.next() else {
            continue;
        };
        let Some(state) = parts.next() else { continue };
        let count: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
        let entry = partition_map
            .entry(partition.to_string())
            .or_insert_with(|| PartitionResources {
                partition: partition.to_string(),
                running_nodes: 0,
                available_nodes: 0,
            });
        match normalize_node_state(state) {
            NormalizedNodeState::Running => entry.running_nodes += count,
            NormalizedNodeState::Available => entry.available_nodes += count,
            NormalizedNodeState::Unavailable => {}
        }
    }
    partition_map.into_values().collect()
}

/// Parse sinfo JSON into sorted partition resource rows.
pub(crate) fn parse_sinfo_resources(value: &serde_json::Value) -> Vec<PartitionResources> {
    let mut partition_map: BTreeMap<String, PartitionResources> = BTreeMap::new();

    let nodes = match value.get("nodes") {
        Some(serde_json::Value::Array(nodes)) => nodes,
        _ => return Vec::new(),
    };

    for node in nodes {
        let Some(partition) = node.get("partition").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(state) = node.get("state").and_then(|v| v.as_str()) else {
            continue;
        };

        let entry = partition_map
            .entry(partition.to_string())
            .or_insert_with(|| PartitionResources {
                partition: partition.to_string(),
                running_nodes: 0,
                available_nodes: 0,
            });

        match normalize_node_state(state) {
            NormalizedNodeState::Running => entry.running_nodes += 1,
            NormalizedNodeState::Available => entry.available_nodes += 1,
            NormalizedNodeState::Unavailable => {} // down/drain/fail/etc — not counted
        }
    }

    partition_map.into_values().collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NormalizedNodeState {
    Running,
    Available,
    Unavailable,
}

fn normalize_node_state(state: &str) -> NormalizedNodeState {
    match state.to_lowercase().as_str() {
        // Running-like states — count as Running
        "allocated" | "alloc" | "mixed" | "mix" => NormalizedNodeState::Running,
        // Idle states — count as Available
        "idle" => NormalizedNodeState::Available,
        // Everything else is unavailable (down, drain, fail, reserved, unknown, etc.)
        _ => NormalizedNodeState::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_allocated_as_running() {
        let json = serde_json::json!({
            "nodes": [
                {"partition": "debug", "state": "allocated"},
                {"partition": "debug", "state": "allocated"}
            ]
        });
        let resources = parse_sinfo_resources(&json);
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
        let resources = parse_sinfo_resources(&json);
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
        let resources = parse_sinfo_resources(&json);
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
        let resources = parse_sinfo_resources(&json);
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
        let resources = parse_sinfo_resources(&json);
        assert_eq!(resources.len(), 3);
        // BTreeMap sorts lexicographically: cpu < debug < gpu
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
        let json = serde_json::json!({"partitions": []});
        let resources = parse_sinfo_resources(&json);
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
        let resources = parse_sinfo_resources(&json);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].running_nodes, 1);
        assert_eq!(resources[0].available_nodes, 0);
    }

    // ── Plain-text parser tests ──

    #[test]
    fn plain_parser_counts_with_node_count_field() {
        let text = "debug alloc 3\ndebug idle 2\n";
        let resources = parse_sinfo_plain(text);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].running_nodes, 3);
        assert_eq!(resources[0].available_nodes, 2);
    }

    #[test]
    fn plain_parser_counts_mix_as_running() {
        let text = "gpu mix 2\ngpu alloc 1\ngpu idle 4\n";
        let resources = parse_sinfo_plain(text);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].running_nodes, 3);
        assert_eq!(resources[0].available_nodes, 4);
    }

    #[test]
    fn plain_parser_defaults_to_1_when_count_missing() {
        let text = "debug alloc\ndebug idle\n";
        let resources = parse_sinfo_plain(text);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].running_nodes, 1);
        assert_eq!(resources[0].available_nodes, 1);
    }

    #[test]
    fn plain_parser_ignores_unavailable_states() {
        let text = "debug down 5\ndebug drain 3\ndebug alloc 2\ndebug idle 1\n";
        let resources = parse_sinfo_plain(text);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].running_nodes, 2);
        assert_eq!(resources[0].available_nodes, 1);
    }

    #[test]
    fn plain_parser_groups_by_partition() {
        let text = "gpu alloc 1\ndebug idle 2\ncpu mix 3\ncpu idle 4\n";
        let resources = parse_sinfo_plain(text);
        assert_eq!(resources.len(), 3);
        // BTreeMap sorts by key: cpu < debug < gpu (before sort_resources)
        assert_eq!(resources[0].partition, "cpu");
        assert_eq!(resources[0].running_nodes, 3);
        assert_eq!(resources[0].available_nodes, 4);
    }

    #[test]
    fn sort_resources_orders_by_available_desc_then_partition() {
        let resources = sort_resources(vec![
            PartitionResources {
                partition: "A".into(),
                running_nodes: 1,
                available_nodes: 5,
            },
            PartitionResources {
                partition: "B".into(),
                running_nodes: 2,
                available_nodes: 10,
            },
            PartitionResources {
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
    fn real_sinfo_integration_diagnostic() {
        let result = fetch_resources();
        match &result {
            Ok(r) => eprintln!("DIAG: fetch_resources OK, {} partitions: {:?}", r.len(), r),
            Err(e) => eprintln!("DIAG: fetch_resources FAILED: {}", e),
        }
        assert!(result.is_ok(), "fetch_resources failed: {:?}", result.err());
        assert!(!result.unwrap().is_empty(), "resources empty");
    }
}
