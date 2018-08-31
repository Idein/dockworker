extern crate dockworker;
extern crate hyper;

use dockworker::{ContainerCreateOptions, ContainerHostConfig, Docker};
use std::str;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let mut host_config = ContainerHostConfig::new();
    host_config.auto_remove(true);
    let mut create = ContainerCreateOptions::new("hello-world:linux");
    create.host_config(host_config);

    let container = docker.create_container(Some("testing"), &create).unwrap();
    docker.start_container(&container.id).unwrap();
    let stream = docker
        .attach_container(&container.id, None, true, true, false, true, false)
        .unwrap();

    for frame in stream {
        match frame {
            Ok(frame) => print!("frame:{}", str::from_utf8(frame.as_bytes()).unwrap()),
            Err(err) => println!("frame:err:{:?}", err),
        }
    }
}
