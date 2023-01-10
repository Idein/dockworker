extern crate dockworker;

use dockworker::{ContainerCreateOptions, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let create = ContainerCreateOptions::new("hello-world:linux");
    let container = docker.create_container(Some("testing"), &create).unwrap();
    println!("{container:?}")
}
