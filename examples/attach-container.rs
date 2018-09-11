extern crate dockworker;
extern crate hyper;

use dockworker::{ContainerCreateOptions, ContainerHostConfig, Docker,
                 container::{self, AttachContainer}};
use std::str;
use std::io::{BufRead, BufReader, Read};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let mut host_config = ContainerHostConfig::new();
    host_config.auto_remove(true);
    let mut create = ContainerCreateOptions::new("hello-world:linux");
    create.host_config(host_config);

    let container = docker.create_container(Some("testing"), &create).unwrap();
    docker.start_container(&container.id).unwrap();
    let mut res = docker
        .attach_container(&container.id, None, true, true, false, true, false)
        .unwrap();
    res.stdin(container::stdio::Stdio::piped())
        .stdout(container::stdio::Stdio::piped())
        .stderr(container::stdio::Stdio::piped());
    let mut cont: AttachContainer = res.into();
    let mut line_reader = BufReader::new(cont.stdout);

    loop {
        let mut line = String::new();
        let size = line_reader.read_line(&mut line).unwrap();
        print!("{:4}: {}", size, line);
        if size == 0 {
            break;
        }
    }
    println!("");
}
