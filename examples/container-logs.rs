extern crate dockworker;

use std::time::Duration;
use std::io::{BufRead, BufReader};

use dockworker::{ContainerCreateOptions, ContainerLogOptions, Docker};
use dockworker::container::LogFollowContainer;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let container_name = "testing"; // "testing";

    docker
        .remove_container(container_name, None, Some(true), None)
        .unwrap();

    let mut create = ContainerCreateOptions::new("alpine:latest");
    create.tty(true);
    create.entrypoint(vec!["/bin/ping".to_string()]);
    create.cmd("localhost".to_string());

    let container = docker
        .create_container(Some(container_name), &create)
        .unwrap();
    docker.start_container(&container.id).unwrap();

    std::thread::sleep(Duration::from_secs(5));

    println!("Container to log: {}", &container.id);
    let log_options = ContainerLogOptions {
        stdout: true,
        stderr: true,
        ..ContainerLogOptions::default()
    };

    let res = docker.log_container(&container.id, &log_options).unwrap();

    println!("Current logs after 5 seconds:");
    let cont: LogFollowContainer = res.into();
    let mut line_reader = BufReader::new(cont.stdout_and_err);

    loop {
        let mut line = String::new();
        match line_reader.read_line(&mut line) {
            Ok(size) => {
                print!("{:4}: {}", size, line);
                if size == 0 {
                    break;
                }
            }
            Err(e) => eprint!("{:?}", e),
        }
    }

    //
    // Follow example:
    //
    println!("Follow logs example:");
    let res = docker.log_container(&container.id, &log_options).unwrap();
    let cont: LogFollowContainer = res.into();

    let mut line_reader = BufReader::new(cont.stdout_and_err);

    loop {
        let mut line = String::new();
        match line_reader.read_line(&mut line) {
            Ok(size) => {
                print!("{:4}: {}", size, line);
                if size == 0 {
                    break;
                }
            }
            Err(_e) => break,
        }
    }

    docker
        .stop_container(&container.id, Duration::from_secs(2))
        .unwrap();
}
