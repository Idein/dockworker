extern crate dockworker;

use dockworker::{ContainerListOptions, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let opts = ContainerListOptions::default();
    for container in docker.containers(opts).unwrap() {
        for stats in docker
            .stats(&container.Id, Some(false), Some(true))
            .unwrap()
        {
            println!("{:#?}", stats.unwrap());
        }
    }
}
