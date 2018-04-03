extern crate boondock;

use boondock::{Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let container = docker.create_container("testing", "openjdk:8").unwrap();
    println!("{:?}", container)
}
