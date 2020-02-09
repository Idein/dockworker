extern crate dockworker;

use dockworker::{network::*, Docker};
use std::collections::HashMap;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    for network in docker.list_networks().unwrap() {
        println!("network: {:?}", network);
    }
    let create = NetworkCreateOptions {
        name: "example_network".to_string(),
        check_duplicate: false,
        driver: "bridge".to_string(),
        internal: true,
        attachable: false,
        ingress: false,
        ipam: IPAM {
            Driver: "default".to_string(),
            Config: vec![],
            Options: vec![],
        },
        enable_ipv6: false,
        options: HashMap::new(),
        labels: HashMap::new(),
    };
    println!("create network: {:?}", create);
    let res = docker.create_network(&create).unwrap();
    println!("res: {:?}", res);
    assert!(docker
        .list_networks()
        .unwrap()
        .iter()
        .find(|n| n.Id == res.Id)
        .is_some());
    println!("remove network: {}", res.Id);
    docker.remove_network(&res.Id).unwrap();
    assert!(docker
        .list_networks()
        .unwrap()
        .iter()
        .find(|n| n.Id == res.Id)
        .is_none());
}
