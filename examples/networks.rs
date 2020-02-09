extern crate dockworker;

use dockworker::{network::*, Docker};
use std::collections::HashMap;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    for network in docker.list_networks(ListNetworkFilters::default()).unwrap() {
        println!(
            "{:20.12}{:25.}{:10.}{:8.}",
            network.Id, network.Name, network.Driver, network.Scope
        );
    }
    let create = NetworkCreateOptions {
        name: "example_network".to_string(),
        check_duplicate: false,
        driver: "bridge".to_string(),
        internal: true,
        attachable: false,
        ingress: false,
        ipam: IPAM::default(),
        enable_ipv6: false,
        options: HashMap::new(),
        labels: HashMap::new(),
    };
    println!(
        "create network: {}",
        serde_json::to_string_pretty(&create).unwrap()
    );
    let res = docker.create_network(&create).unwrap();
    println!("res: {:?}", res);
    let mut filter = ListNetworkFilters::default();
    filter.id(res.Id.as_str().into());
    println!("remove network: {}", res.Id);
    docker.remove_network(&res.Id).unwrap();
}
