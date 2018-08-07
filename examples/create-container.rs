extern crate boondock;

use boondock::{Docker, ContainerCreateOptions};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let create = ContainerCreateOptions::new("hello-world:linux");
    let container = docker.create_container("testing", &create).unwrap();
    println!("{:?}", container)
}
