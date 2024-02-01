use dockworker::{ContainerCreateOptions, Docker, ExporsedPorts, PortBindings};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let mut create = ContainerCreateOptions::new("nginx:latest");
    create.tty(true);
    create.exposed_ports(ExporsedPorts(vec![(80, "tcp".to_string())]));
    create.port_bindings(PortBindings(vec![(80, "tcp".to_string(), 8080)]));
    let container = docker
        .create_container(Some("testing"), &create)
        .await
        .unwrap();
    docker.start_container(&container.id).await.unwrap();
}
