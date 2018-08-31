extern crate dockworker;

use dockworker::{ContainerCreateOptions, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let mut create = ContainerCreateOptions::new("hello-world:linux");
    create.tty(true);
    let container = docker.create_container(Some("testing"), &create).unwrap();
    docker.start_container(&container.id).unwrap();
}
