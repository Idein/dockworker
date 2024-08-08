// rename-container.rs, adapted from start-container.rs
use dockworker::{ContainerCreateOptions, Docker};
use dockworker::container::ContainerFilters;

#[tokio::main]
async fn main() {
    let name = "hello-world";
    let mut filters = ContainerFilters::new();
    filters.status(dockworker::container::ContainerStatus::Created);
    filters.name(name);
    let docker = Docker::connect_with_defaults().unwrap();
    let mut create = ContainerCreateOptions::new("hello-world:linux");
    create.tty(true);
    let container = docker
        .create_container(None, &create)
        .await
        .unwrap();
    docker.rename_container(&container.id, name)
        .await
        .unwrap();
    let containers = docker.list_containers(None, None, None, filters).await.unwrap();
    for container in containers {
        for name in container.Names {
            println!("Found container with name: {}", name);
        }
    }
}
