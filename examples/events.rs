extern crate dockworker;

use dockworker::{ContainerCreateOptions, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let events = docker.events(None, None, None).unwrap();

    let create = ContainerCreateOptions::new("hello-world:linux");
    docker
        .create_image("hello-world", "linux")
        .unwrap()
        .for_each(drop);
    let container = docker.create_container(None, &create).unwrap();
    docker.start_container(&container.id).unwrap();

    events
        .map(|e| e.unwrap())
        .map(|e| {
            println!("{e:?}");
            e
        })
        .find(|e| e.Type == "network" && e.Action == "disconnect");

    docker
        .remove_container(&container.id, None, Some(true), None)
        .unwrap();
}
