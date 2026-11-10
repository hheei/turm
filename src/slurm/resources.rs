use super::SlurmClient;
use crate::app::PartitionResources;
use std::collections::BTreeMap;
use std::process::Command;

// ── sinfo fetching ──

/// Runs `sinfo` and returns parsed partition resources.
pub(crate) fn fetch_resources(client: &SlurmClient) -> Result<Vec<PartitionResources>, String> {
    let output = client
        .output(Command::new(&client.sinfo).args(["-o", "%P|%t|%D|%c|%G|%l|%m", "--noheader"]))
        .map_err(|error| error.to_string())?;
    let mut resources = parse_sinfo_plain(&output);
    let group = client.primary_group()?;
    let output = client
        .output(Command::new(&client.squeue).args([
            "--noheader",
            "--states=RUNNING",
            &format!("--accounts={group}"),
            "--format=%P|%D",
        ]))
        .map_err(|error| error.to_string())?;
    let group_usage = parse_group_usage(&output);
    for resource in &mut resources {
        resource.group_used_nodes = group_usage
            .get(&resource.partition)
            .copied()
            .unwrap_or_default()
            .min(resource.running_nodes);
    }
    Ok(sort_resources(resources))
}

pub(crate) fn fetch_resources_with(
    sinfo: impl AsRef<std::ffi::OsStr>,
    squeue: impl AsRef<std::ffi::OsStr>,
) -> Result<Vec<PartitionResources>, String> {
    let mut client = SlurmClient::default();
    client.sinfo = sinfo.as_ref().into();
    client.squeue = squeue.as_ref().into();
    fetch_resources(&client)
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

/// Parse plain `sinfo -o '%P|%t|%D|%c|%G|%l|%m' --noheader` output.
pub(crate) fn parse_sinfo_plain(text: &str) -> Vec<PartitionResources> {
    let mut partition_map: BTreeMap<String, PartitionResources> = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields = line.split('|').collect::<Vec<_>>();
        let (partition, state, count, details) = if fields.len() >= 7 {
            (
                fields[0].trim(),
                fields[1].trim(),
                fields[2].trim().parse().unwrap_or(1),
                Some([
                    fields[3].trim(),
                    fields[4].trim(),
                    fields[5].trim(),
                    fields[6].trim(),
                ]),
            )
        } else {
            let mut parts = line.split_whitespace();
            let Some(partition) = parts.next() else {
                continue;
            };
            let Some(state) = parts.next() else { continue };
            (
                partition,
                state,
                parts.next().and_then(|s| s.parse().ok()).unwrap_or(1),
                None,
            )
        };
        let entry = partition_map
            .entry(partition.to_string())
            .or_insert_with(|| new_partition_resources(partition));
        entry.total_nodes = entry.total_nodes.saturating_add(count);
        if let Some([cpus, gpus, time_limit, memory]) = details {
            merge_detail(&mut entry.cpus_per_node, cpus);
            merge_detail(&mut entry.gpus_per_node, gpus);
            merge_detail(&mut entry.time_limit, time_limit);
            merge_memory_detail(&mut entry.memory_per_node, memory);
        }
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
            .or_insert_with(|| new_partition_resources(partition));

        entry.total_nodes = entry.total_nodes.saturating_add(1);
        match normalize_node_state(state) {
            NormalizedNodeState::Running => entry.running_nodes += 1,
            NormalizedNodeState::Available => entry.available_nodes += 1,
            NormalizedNodeState::Unavailable => {} // down/drain/fail/etc — not counted
        }
    }

    partition_map.into_values().collect()
}

fn new_partition_resources(partition: &str) -> PartitionResources {
    PartitionResources {
        partition: partition.to_string(),
        total_nodes: 0,
        running_nodes: 0,
        group_used_nodes: 0,
        available_nodes: 0,
        cpus_per_node: "N/A".to_string(),
        gpus_per_node: "N/A".to_string(),
        time_limit: "N/A".to_string(),
        memory_per_node: "N/A".to_string(),
    }
}

fn merge_detail(current: &mut String, value: &str) {
    let value = match value {
        "" | "(null)" | "N/A" => return,
        value => value.to_string(),
    };
    if current == "N/A" {
        *current = value;
    } else if *current != value {
        *current = "mixed".to_string();
    }
}

fn merge_memory_detail(current: &mut String, value: &str) {
    if current != "N/A" {
        return;
    }
    if let Some(value) = format_memory(value) {
        *current = value;
    }
}

fn format_memory(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("N/A") || value == "(null)" {
        return None;
    }
    let split_at = value
        .find(|character: char| character.is_ascii_alphabetic())
        .unwrap_or(value.len());
    let number = value[..split_at].trim().parse::<f64>().ok()?;
    let unit = value[split_at..].trim().to_ascii_uppercase();
    let megabytes = match unit.as_str() {
        "" | "M" | "MB" => number,
        "K" | "KB" => number / 1_000.0,
        "G" | "GB" => number * 1_000.0,
        "T" | "TB" => number * 1_000_000.0,
        _ => return Some(value.to_string()),
    };
    if megabytes >= 1_000.0 {
        Some(format_memory_value(megabytes / 1_000.0, "G"))
    } else {
        Some(format_memory_value(megabytes, "M"))
    }
}

fn format_memory_value(value: f64, unit: &str) -> String {
    let formatted = format!("{value:.1}");
    format!("{}{}", formatted.trim_end_matches(".0"), unit)
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
