extern crate dockworker;

use dockworker::{container::ContainerFilters, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    for container in docker
        .list_containers(None, None, None, ContainerFilters::default())
        .unwrap()
    {
        for stats in docker
            .stats(&container.Id, Some(false), Some(true))
            .unwrap()
        {
            println!("{:#?}", stats.unwrap());
        }
    }
}
