extern crate dockworker;

use dockworker::container::ContainerFilters;
use dockworker::Docker;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    if let Some(container) = docker
        .list_containers(None, None, None, ContainerFilters::default())
        .unwrap()
        .get(0)
    {
        for stats in docker.stats(container.Id.as_str()).unwrap() {
            println!("{:#?}", stats);
        }
    }
}
