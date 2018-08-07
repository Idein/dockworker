extern crate boondock;

use boondock::{ContainerCreateOptions, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let create = ContainerCreateOptions::new("hello-world:linux");

    let container = docker.create_container("testing", &create).unwrap();
    docker.start_container(&container.id).unwrap();
    let res = docker.attach_container(&container.id, None, true, true, false, true, false).unwrap();
    println!("response: {:?}", res);
}
