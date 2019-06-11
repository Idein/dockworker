extern crate dockworker;
extern crate env_logger;
extern crate hyper;

use dockworker::{container::AttachContainer, ContainerCreateOptions, ContainerHostConfig, Docker};
use std::io::{BufRead, BufReader};

fn main() {
    env_logger::init();
    let docker = Docker::connect_with_defaults().unwrap();
    let mut host_config = ContainerHostConfig::new();
    host_config.auto_remove(true);
    let mut create = ContainerCreateOptions::new("bash:latest");
    create.host_config(host_config);
    create.cmd("-c".to_string());
    create.cmd(r#"yes | awk '{print "[\"yes\"]"}'"#.to_owned());

    let container = docker.create_container(None, &create).unwrap();
    docker.start_container(&container.id).unwrap();
    let res = docker
        .attach_container(&container.id, None, true, true, false, true, false)
        .unwrap();
    let cont: AttachContainer = res.into();
    let mut line_reader = BufReader::new(cont.stdout);

    for i in 0..100 {
        let mut line = String::new();
        let size = line_reader.read_line(&mut line).unwrap();
        print!("{:4}: {:4}: {}", i, size, line);
        if size == 0 {
            break;
        }
    }
    println!("");
}
