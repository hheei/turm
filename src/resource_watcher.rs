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
/// Tries `--json` first (Slurm 23.02+), falls back to plain `-o '%P %t' --noheader`.
pub(crate) fn fetch_resources() -> Result<Vec<PartitionResources>, Box<dyn std::error::Error>> {
    // Try --json first (newer Slurm)
    if let Ok(output) = Command::new("sinfo").arg("--json").output() {
        if output.status.success() {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                return Ok(parse_sinfo_resources(&value));
            }
        }
    }
    // Fallback: plain sinfo output (Slurm 20.11 and older)
    let output = Command::new("sinfo")
        .args(["-o", "%P %t", "--noheader"])
        .output()?;
    if !output.status.success() {
        return Err("sinfo command failed".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_sinfo_plain(&text))
}

/// Parse plain `sinfo -o '%P %t' --noheader` output: "partition state" per line.
pub(crate) fn parse_sinfo_plain(text: &str) -> Vec<PartitionResources> {
    let mut partition_map: BTreeMap<String, PartitionResources> = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((partition, state)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let state = state.trim();
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
    fn plain_parser_counts_alloc_as_running() {
        let text = "debug alloc\ndebug alloc\ndebug idle\n";
        let resources = parse_sinfo_plain(text);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].running_nodes, 2);
        assert_eq!(resources[0].available_nodes, 1);
    }

    #[test]
    fn plain_parser_counts_mix_as_running() {
        let text = "gpu mix\ngpu alloc\ngpu idle\n";
        let resources = parse_sinfo_plain(text);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].running_nodes, 2);
        assert_eq!(resources[0].available_nodes, 1);
    }

    #[test]
    fn plain_parser_ignores_unknown_states() {
        let text = "debug down\ndebug drain\ndebug alloc\ndebug idle\n";
        let resources = parse_sinfo_plain(text);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].running_nodes, 1);
        assert_eq!(resources[0].available_nodes, 1);
    }

    #[test]
    fn plain_parser_groups_and_sorts_by_partition() {
        let text = "gpu alloc\ndebug idle\ncpu mix\ncpu idle\n";
        let resources = parse_sinfo_plain(text);
        assert_eq!(resources.len(), 3);
        assert_eq!(resources[0].partition, "cpu");
        assert_eq!(resources[0].running_nodes, 1);
        assert_eq!(resources[0].available_nodes, 1);
        assert_eq!(resources[1].partition, "debug");
        assert_eq!(resources[2].partition, "gpu");
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
