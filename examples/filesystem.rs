extern crate dockworker;

use dockworker::{container::ContainerFilters, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    if let Some(container) = docker
        .list_containers(None, None, None, ContainerFilters::default())
        .unwrap()
        .get(0)
    {
        for change in docker.filesystem_changes(container.Id.as_str()).unwrap() {
            println!("{change:#?}");
        }
    }
}
