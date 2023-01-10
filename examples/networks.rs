extern crate dockworker;

use dockworker::{network::*, Docker};
use std::net::Ipv4Addr;

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    for network in docker
        .list_networks(ListNetworkFilters::default())
        .await
        .unwrap()
    {
        println!(
            "{:20.12}{:25}{:10}{:8}",
            network.Id, network.Name, network.Driver, network.Scope
        );
    }
    let create = {
        let mut opt = NetworkCreateOptions::new("example_network");
        opt.enable_icc()
            .enable_ip_masquerade()
            .host_binding_ipv4(Ipv4Addr::new(0, 0, 0, 0))
            .bridge_name("dockworker_ex_0")
            .driver_mtu(1500);
        opt.internal = true;
        opt
    };

    println!(
        "create network: {}",
        serde_json::to_string_pretty(&create).unwrap()
    );
    let res = docker.create_network(&create).await.unwrap();
    println!("res: {res:?}");
    let mut filter = ListNetworkFilters::default();
    filter.id(res.Id.as_str().into());
    println!("remove network: {}", res.Id);
    docker.remove_network(&res.Id).await.unwrap();
}
