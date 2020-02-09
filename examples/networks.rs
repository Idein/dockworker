extern crate dockworker;

use dockworker::Docker;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    for network in docker.list_networks().unwrap() {
        println!("network: {:?}", network);
    }
}
