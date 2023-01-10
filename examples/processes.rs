extern crate dockworker;

use dockworker::{container::ContainerFilters, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    if let Some(container) = docker
        .list_containers(None, None, None, ContainerFilters::default())
        .unwrap()
        .get(0)
    {
        for process in docker.processes(container.Id.as_str()).unwrap() {
            println!("{process:#?}");
        }
    }
}
