extern crate dockworker;

use dockworker::{container::ContainerFilters, Docker};
use std::fs::File;
use std::io;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let mut file = File::create("temp.tar").unwrap();
    if let Some(container) = docker
        .list_containers(None, None, None, ContainerFilters::default())
        .unwrap()
        .get(0)
    {
        let mut res = docker.export_container(container.Id.as_str()).unwrap();
        io::copy(&mut res, &mut file).unwrap();
    }
}
