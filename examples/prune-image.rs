extern crate dockworker;

use dockworker::{container::ContainerFilters, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();

    let prunedt = docker.prune_image(true).unwrap();
    println!("pruned(true): {prunedt:?}");

    let prunedf = docker.prune_image(false).unwrap();
    println!("pruned(false): {prunedf:?}");

    let containers = docker
        .list_containers(Some(true), None, None, ContainerFilters::new())
        .unwrap();

    containers.iter().for_each(|c| {
        println!("image: {}", c.Image);
    });
}
