extern crate dockworker;

use std::time::Duration;
use std::io::BufRead;
use dockworker::{ContainerCreateOptions, ContainerLogOptions, Docker};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();

    docker.remove_container("testing", None, Some(true), None).unwrap();

    let mut create = ContainerCreateOptions::new("alpine:latest");
    create.tty(true);
    create.entrypoint(vec!["/bin/ping".to_string()]);
    create.cmd("localhost".to_string());

    let container = docker.create_container(Some("testing"), &create).unwrap();
    docker.start_container(&container.id).unwrap();

    std::thread::sleep(Duration::from_secs(5));

    println!("Container to log: {}", &container.id);
    let log_options = ContainerLogOptions {
        stdout: true,
        stderr: true,
        since: None,
        timestamps: None,
        tail: None
    };

    let logs = docker.log_container(&container.id, &log_options);

    println!("Current logs after 5 seconds:");
    for line in logs.unwrap().lines() {
        println!("{}", line);
    }

    //
    // Follow example:
    //
    println!("Follow logs example:");
    let log_stream = docker.log_container_and_follow(&container.id, &log_options);

    let mut reader = std::io::BufReader::new(log_stream.unwrap());
    let mut size = 1;
    let mut line_buffer = String::new();
    while size > 0 {
        match reader.read_line(&mut line_buffer) {
            Ok(s) => {
                size = s;
                print!("{}", line_buffer);
                line_buffer.clear();
            }
            Err(e) => { println!("{:?}", e); break; }
        }
    }

    docker.stop_container(&container.id, Duration::from_secs(2)).unwrap();
}
