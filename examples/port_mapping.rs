use dockworker::{ContainerCreateOptions, Docker, ExporsedPorts, HostConfig, PortBindings};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let mut create = ContainerCreateOptions::new("nginx:latest");
    create.tty(true);
    create.exposed_ports(ExporsedPorts(vec![(80, "tcp".to_string())]));
    create.host_config(HostConfig {
        port_bindings: PortBindings(vec![(80, "tcp".to_string(), 8080)]),
    });
    let container = docker
        .create_container(Some("test"), &create)
        .await
        .unwrap();
    docker.start_container(&container.id).await.unwrap();
}
