extern crate dockworker;

use std::io::{BufRead, BufReader};

use dockworker::{ContainerCreateOptions, ContainerLogOptions, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();

    let mut create = ContainerCreateOptions::new("alpine:latest");
    create.tty(true);
    create.entrypoint(vec!["/bin/ping".into(), "-c".into(), "5".into()]);
    create.cmd("localhost".to_string());

    let container = docker.create_container(None, &create).unwrap();
    docker.start_container(&container.id).unwrap();

    println!("Container to log: {}", &container.id);
    let log_options = ContainerLogOptions {
        stdout: true,
        stderr: true,
        follow: true,
        ..ContainerLogOptions::default()
    };

    let res = docker.log_container(&container.id, &log_options).unwrap();
    let mut lines = BufReader::new(res).lines();

    while let Some(line) = lines.next() {
        match line {
            Ok(line) => println!("read: {}", line),
            Err(e) => eprintln!("err: {:?}", e),
        }
    }
    println!(""); // line break

    // already stopped
    // docker
    //     .stop_container(&container.id, Duration::from_secs(2))
    //     .unwrap();
    docker
        .remove_container(&container.id, None, Some(true), None)
        .unwrap();
}
