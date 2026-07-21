use super::*;

pub fn parse_sinfo_json(value: &serde_json::Value) -> Vec<ResourceSnapshot> {
    crate::resource_watcher::parse_sinfo_resources(value)
        .into_iter()
        .map(ResourceSnapshot::from)
        .collect()
}

pub fn parse_sinfo_text(value: &str) -> Vec<ResourceSnapshot> {
    crate::resource_watcher::parse_sinfo_plain(value)
        .into_iter()
        .map(ResourceSnapshot::from)
        .collect()
}

pub fn sort_resource_snapshots(resources: Vec<ResourceSnapshot>) -> Vec<ResourceSnapshot> {
    crate::resource_watcher::sort_resources(
        resources
            .into_iter()
            .map(PartitionResources::from)
            .collect(),
    )
    .into_iter()
    .map(ResourceSnapshot::from)
    .collect()
}

pub fn parse_group_usage_text(value: &str) -> Vec<(String, u32)> {
    crate::resource_watcher::parse_group_usage(value)
        .into_iter()
        .collect()
}

pub fn fetch_resources_from(sinfo: &Path) -> Result<Vec<ResourceSnapshot>, String> {
    let squeue = sinfo.with_file_name("squeue");
    crate::resource_watcher::fetch_resources_with(sinfo, squeue)
        .map(|resources| resources.into_iter().map(ResourceSnapshot::from).collect())
        .map_err(|error| error.to_string())
}

impl From<PartitionResources> for ResourceSnapshot {
    fn from(resource: PartitionResources) -> Self {
        Self {
            partition: resource.partition,
            running_nodes: resource.running_nodes,
            group_used_nodes: resource.group_used_nodes,
            available_nodes: resource.available_nodes,
        }
    }
}

impl From<ResourceSnapshot> for PartitionResources {
    fn from(resource: ResourceSnapshot) -> Self {
        Self {
            partition: resource.partition,
            running_nodes: resource.running_nodes,
            group_used_nodes: resource.group_used_nodes,
            available_nodes: resource.available_nodes,
        }
    }
}
