extern crate dockworker;

use dockworker::{container::ContainerFilters, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let filter = ContainerFilters::new();
    let containers = docker.list_containers(None, None, None, filter).unwrap();

    containers.iter().for_each(|c| {
        println!("{c:?}");
    });
}
