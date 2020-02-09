#![cfg(test)]

use super::container::HealthState;
use super::ImageLayer;
use container::{Container, ContainerInfo};
use filesystem::FilesystemChange;
use hyper_client::Response;
use image::{Image, SummaryImage};
use network::Network;
use process::Top;
use serde_json;
use stats::{Stats, StatsReader};
use system::SystemInfo;
use version::Version;

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
fn get_stats_single() {
    let response = get_stats_single_event(1);
    print!("{}", response);
    assert!(serde_json::from_str::<Stats>(&response).is_ok())
}

#[test]
fn get_stats_streaming() {
    let response = get_stats_response();
    let mut reader = StatsReader::new(response);

    let stats = reader.next().unwrap().unwrap();
    assert_eq!(&stats.read, "2015-04-09T07:02:08.480022081Z");

    let stats = reader.next().unwrap().unwrap();
    assert_eq!(&stats.read, "2015-04-09T07:02:08.480022082Z");

    let stats = reader.next().unwrap().unwrap();
    assert_eq!(&stats.read, "2015-04-09T07:02:08.480022083Z");

    assert!(reader.next().is_none());
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
    assert!(serde_json::from_str::<ContainerInfo>(&response).is_ok());

    let response = get_container_info_response_with_healthcheck();
    assert!(serde_json::from_str::<ContainerInfo>(&response).is_ok());
}

#[test]
fn get_healthcheck_info() {
    let response = get_container_info_response_with_healthcheck();
    let container_info = serde_json::from_str::<ContainerInfo>(&response).unwrap();
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

fn get_stats_response() -> Response {
    let mut response = http::Response::builder();
    response.status(http::StatusCode::OK);
    response.header("Transfer-Encoding", "chunked");
    response.header("Connection", "Close");
    let s1 = get_stats_single_event(1);
    let s2 = get_stats_single_event(2);
    let s3 = get_stats_single_event(3);
    Response::new(
        response
            .body(hyper::Body::from(format!("{}\n{}\n{}", s1, s2, s3)))
            .unwrap(),
    )
}

fn get_stats_single_event(n: u64) -> String {
    let template = include_str!("fixtures/stats_single_event.json")
        .to_string()
        .replace("\n", "");
    template.replace("{}", &n.to_string())
}
