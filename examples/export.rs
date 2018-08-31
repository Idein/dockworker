extern crate dockworker;

use dockworker::{ContainerListOptions, Docker};
use std::fs::File;
use std::io;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let opts = ContainerListOptions::default();
    let mut file = File::create("temp.tar").unwrap();
    if let Some(container) = docker.containers(opts).unwrap().get(0) {
        let mut res = docker.export_container(container).unwrap();
        io::copy(&mut res, &mut file).unwrap();
    }
}
