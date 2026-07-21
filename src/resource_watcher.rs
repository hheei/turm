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
    fetch_resources_with("sinfo")
}

pub(crate) fn fetch_resources_with(
    sinfo: impl AsRef<std::ffi::OsStr>,
) -> Result<Vec<PartitionResources>, Box<dyn std::error::Error>> {
    let sinfo = sinfo.as_ref();
    // Try --json first (newer Slurm)
    if let Ok(output) = Command::new(sinfo).arg("--json").output() {
        if output.status.success() {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                return Ok(sort_resources(parse_sinfo_resources(&value)));
            }
        }
    }
    // Fallback: plain sinfo output (Slurm 20.11 and older)
    let output = Command::new(sinfo)
        .args(["-o", "%P %t %D", "--noheader"])
        .output()?;
    if !output.status.success() {
        return Err("sinfo command failed".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(sort_resources(parse_sinfo_plain(&text)))
}

/// Sort resources: Available descending, then partition name ascending.
pub(crate) fn sort_resources(mut resources: Vec<PartitionResources>) -> Vec<PartitionResources> {
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
