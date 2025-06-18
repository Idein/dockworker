#![cfg(test)]

use crate::container::{Container, ContainerInfo, HealthState};
use crate::filesystem::FilesystemChange;
use crate::image::{Image, SummaryImage};
use crate::network::Network;
use crate::options::ImageLayer;
use crate::process::Top;
use crate::stats::Stats;
use crate::system::SystemInfo;
use crate::version::Version;

#[test]
fn get_containers() {
    let response = get_containers_response();
    assert!(serde_json::from_str::<Vec<Container>>(response).is_ok())
}

#[test]
fn get_networks() {
    let response = include_str!("fixtures/list_networks.json");
    assert!(serde_json::from_str::<Vec<Network>>(response).is_ok())
}

#[test]
fn get_stats_suspended() {
    let stats_oneshot = include_str!("fixtures/stats_suspend.json");
    let v = serde_json::from_str::<Stats>(stats_oneshot).unwrap();
    assert!(v.memory_stats.is_none());
}

#[tokio::test]
async fn get_stats_streaming() {
    let res = get_stats_response();
    let src = crate::docker::into_jsonlines::<Stats>(res.into_body()).unwrap();
    use futures::stream::StreamExt;
    let stats = src
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(stats.len(), 3);
    assert!(stats[0].memory_stats.is_some());
    assert!(stats[1].memory_stats.is_some());
    assert!(stats[2].memory_stats.is_some());
}

#[test]
fn get_system_info() {
    let response = get_system_info_response();
    assert!(serde_json::from_str::<SystemInfo>(response).is_ok())
}

#[test]
fn get_image_list() {
    let response = get_image_list_response();
    let images: Vec<SummaryImage> = serde_json::from_str(response).unwrap();
    assert_eq!(3, images.len());
}

#[test]
fn get_image() {
    let response = get_image_response();
    println!("response: {:?}", serde_json::from_str::<Image>(response));
}

#[test]
fn get_image_history() {
    let response = get_image_history_reponse();
    let images: Vec<ImageLayer> = serde_json::from_str(response).unwrap();
    assert_ne!(images[0].id, None);
    assert_eq!(2, images.len());
}

#[test]
fn get_container_info() {
    let response = get_container_info_response();
    serde_json::from_str::<ContainerInfo>(response).unwrap();

    let response = get_container_info_response_with_healthcheck();
    serde_json::from_str::<ContainerInfo>(response).unwrap();
}

#[test]
fn get_healthcheck_info() {
    let response = get_container_info_response_with_healthcheck();
    let container_info = serde_json::from_str::<ContainerInfo>(response).unwrap();
    assert!(container_info.State.Health.is_some());
    assert!(container_info.State.Health.unwrap().Status == HealthState::Healthy);
}

#[test]
fn get_processes() {
    let response = get_processes_response();
    assert!(serde_json::from_str::<Top>(response).is_ok())
}

#[test]
fn get_filesystem_changes() {
    let response = get_filesystem_changes_response();
    assert!(serde_json::from_str::<Vec<FilesystemChange>>(response).is_ok())
}

#[test]
fn get_version() {
    let response = get_version_response();
    assert!(serde_json::from_str::<Version>(response).is_ok())
}

fn get_containers_response() -> &'static str {
    include_str!("fixtures/containers_response.json")
}

fn get_system_info_response() -> &'static str {
    include_str!("fixtures/system_info.json")
}

// `docker inspect debian:wheely-2019- |  jq '.[]'
fn get_image_response() -> &'static str {
    include_str!("fixtures/image.json")
}

fn get_image_list_response() -> &'static str {
    include_str!("fixtures/image_list.json")
}

fn get_image_history_reponse() -> &'static str {
    // First has Id, second has Id missing.
    include_str!("fixtures/image_history.json")
}

fn get_container_info_response() -> &'static str {
    include_str!("fixtures/container_inspect.json")
}

fn get_container_info_response_with_healthcheck() -> &'static str {
    include_str!("fixtures/container_inspect_health.json")
}

fn get_processes_response() -> &'static str {
    include_str!("fixtures/processes.json")
}

fn get_filesystem_changes_response() -> &'static str {
    include_str!("fixtures/filesystem_changes.json")
}

fn get_version_response() -> &'static str {
    include_str!("fixtures/version.json")
}

fn get_stats_response() -> http::Response<hyper::Body> {
    let response = http::Response::builder()
        .status(http::StatusCode::OK)
        .header("Transfer-Encoding", "chunked")
        .header("Connection", "Close");
    let body = include_str!("fixtures/stats_stream.json").to_string();
    response.body(hyper::Body::from(body)).unwrap()
}

#[test]
fn container_prune_filters_empty() {
    use crate::options::ContainerPruneFilters;
    let filters = ContainerPruneFilters::new();
    assert!(filters.is_empty());
    assert!(filters.until.is_empty());
    assert!(filters.label.is_empty());
}

#[test]
fn container_prune_filters_until() {
    use crate::options::ContainerPruneFilters;
    let mut filters = ContainerPruneFilters::new();
    filters.until("24h".to_string());
    filters.until("1h".to_string());
    
    assert!(!filters.is_empty());
    assert_eq!(filters.until, vec!["24h", "1h"]);
    assert!(filters.label.is_empty());
}

#[test]
fn container_prune_filters_label() {
    use crate::options::ContainerPruneFilters;
    let mut filters = ContainerPruneFilters::new();
    filters.label("test=example".to_string());
    filters.label("version=1.0".to_string());
    
    assert!(!filters.is_empty());
    assert_eq!(filters.label, vec!["test=example", "version=1.0"]);
    assert!(filters.until.is_empty());
}

#[test]
fn container_prune_filters_mixed() {
    use crate::options::ContainerPruneFilters;
    let mut filters = ContainerPruneFilters::new();
    filters.until("24h".to_string());
    filters.label("test=example".to_string());
    
    assert!(!filters.is_empty());
    assert_eq!(filters.until, vec!["24h"]);
    assert_eq!(filters.label, vec!["test=example"]);
}

#[test]
fn container_prune_filters_serialization() {
    use crate::options::ContainerPruneFilters;
    let mut filters = ContainerPruneFilters::new();
    filters.until("24h".to_string());
    filters.label("test=example".to_string());
    
    let json = serde_json::to_string(&filters).unwrap();
    assert!(json.contains("until"));
    assert!(json.contains("label"));
    assert!(json.contains("24h"));
    assert!(json.contains("test=example"));
}

#[test]
fn container_prune_filters_empty_serialization() {
    use crate::options::ContainerPruneFilters;
    let filters = ContainerPruneFilters::new();
    
    let json = serde_json::to_string(&filters).unwrap();
    assert_eq!(json, "{}");
}

#[test]
fn pruned_containers_deserialization() {
    use crate::options::PrunedContainers;
    let json = r#"{"ContainersDeleted":["container1","container2"],"SpaceReclaimed":1024}"#;
    
    let result: PrunedContainers = serde_json::from_str(json).unwrap();
    assert_eq!(result.containers_deleted, vec!["container1", "container2"]);
    assert_eq!(result.space_reclaimed, 1024);
}
