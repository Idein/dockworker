extern crate dockworker;

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::iter::FromIterator;

use dockworker::{
    ContainerCreateOptions, ContainerHostConfig, ContainerLogOptions, Docker, LogConfig,
};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();

    let mut create = ContainerCreateOptions::new("alpine:latest");
    create.tty(true);
    create.entrypoint(vec!["/bin/ping".into(), "-c".into(), "5".into()]);
    create.cmd("localhost".to_string());
    create.host_config({
        let mut host = ContainerHostConfig::new();
        let log_config = LogConfig {
            config: HashMap::from_iter(
                vec![("tag".to_string(), "dockworker-test".to_string())].into_iter(),
            ),
            ..Default::default()
        };
        println!("logging with: {log_config:?}");
        host.log_config(log_config);
        host
    });

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
    let lines = BufReader::new(res).lines();

    for line in lines {
        match line {
            Ok(line) => println!("read: {line}"),
            Err(e) => eprintln!("err: {e:?}"),
        }
    }
    println!(); // line break

    // already stopped
    // docker
    //     .stop_container(&container.id, Duration::from_secs(2))
    //     .unwrap();
    docker
        .remove_container(&container.id, None, Some(true), None)
        .unwrap();
}
