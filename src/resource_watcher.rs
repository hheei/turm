use std::collections::BTreeMap;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crossbeam::channel::Sender;

use crate::app::AppMessage;
use crate::app::PartitionResources;
use crate::squeue_args::primary_group;

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
pub(crate) fn fetch_resources() -> Result<Vec<PartitionResources>, Box<dyn std::error::Error>> {
    fetch_resources_with("sinfo", "squeue")
}

pub(crate) fn fetch_resources_with(
    sinfo: impl AsRef<std::ffi::OsStr>,
    squeue: impl AsRef<std::ffi::OsStr>,
) -> Result<Vec<PartitionResources>, Box<dyn std::error::Error>> {
    let sinfo = sinfo.as_ref();
    let output = Command::new(sinfo)
        .args(["-o", "%P %t %D", "--noheader"])
        .output()?;
    if !output.status.success() {
        return Err("sinfo command failed".into());
    }
    let mut resources = parse_sinfo_plain(&String::from_utf8_lossy(&output.stdout));
    let group = primary_group().ok_or("failed to determine primary group")?;
    let output = Command::new(squeue)
        .args([
            "--noheader",
            "--states=RUNNING",
            &format!("--accounts={group}"),
            "--format=%P|%D",
        ])
        .output()?;
    if !output.status.success() {
        return Err("squeue command failed".into());
    }
    let group_usage = parse_group_usage(&String::from_utf8_lossy(&output.stdout));
    for resource in &mut resources {
        resource.group_used_nodes = group_usage
            .get(&resource.partition)
            .copied()
            .unwrap_or_default()
            .min(resource.running_nodes);
    }
    Ok(sort_resources(resources))
}

/// Sort resources: total nodes descending, then available nodes descending.
pub(crate) fn sort_resources(mut resources: Vec<PartitionResources>) -> Vec<PartitionResources> {
    resources.sort_by(|a, b| {
        b.total_nodes
            .cmp(&a.total_nodes)
            .then_with(|| b.available_nodes.cmp(&a.available_nodes))
            .then_with(|| a.partition.cmp(&b.partition))
    });
    resources
}

pub(crate) fn parse_group_usage(text: &str) -> BTreeMap<String, u32> {
    let mut usage = BTreeMap::new();
    for line in text.lines() {
        let Some((partition, count)) = line.trim().split_once('|') else {
            continue;
        };
        let Ok(count) = count.trim().parse::<u32>() else {
            continue;
        };
        *usage.entry(partition.trim().to_string()).or_default() += count;
    }
    usage
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
                total_nodes: 0,
                running_nodes: 0,
                group_used_nodes: 0,
                available_nodes: 0,
            });
        entry.total_nodes = entry.total_nodes.saturating_add(count);
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
                total_nodes: 0,
                running_nodes: 0,
                group_used_nodes: 0,
                available_nodes: 0,
            });

        entry.total_nodes = entry.total_nodes.saturating_add(1);
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
