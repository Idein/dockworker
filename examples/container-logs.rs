use std::collections::HashMap;
use std::iter::FromIterator;

use dockworker::{
    ContainerCreateOptions, ContainerHostConfig, ContainerLogOptions, Docker, LogConfig,
};
#[tokio::main]
async fn main() {
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

    let container = docker.create_container(None, &create).await.unwrap();
    docker.start_container(&container.id).await.unwrap();

    println!("Container to log: {}", &container.id);
    let log_options = ContainerLogOptions {
        stdout: true,
        stderr: true,
        follow: true,
        ..ContainerLogOptions::default()
    };

    let mut res = docker
        .log_container(&container.id, &log_options)
        .await
        .unwrap();

    use futures::stream::StreamExt;
    while let Some(line) = res.next().await {
        match line {
            Ok(line) => println!("read: {line}"),
            Err(e) => eprintln!("err: {e:?}"),
        }
    }
    println!(); // line break

    // already stopped
    // docker
    //     .stop_container(&container.id, Duration::from_secs(2))
    //     .await
    //     .unwrap();
    docker
        .remove_container(&container.id, None, Some(true), None)
        .await
        .unwrap();
}
