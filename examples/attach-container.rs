extern crate boondock;
extern crate hyper;

use boondock::{ContainerCreateOptions, Docker};
use std::str;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let create = ContainerCreateOptions::new("hello-world:linux");

    let container = docker.create_container("testing", &create).unwrap();
    docker.start_container(&container.id).unwrap();
    let stream = docker
        .attach_container(&container.id, None, true, true, false, true, false)
        .unwrap();

    for frame in stream {
        print!(
            "frame:{}",
            str::from_utf8(frame.unwrap().as_bytes()).unwrap()
        );
    }
}
