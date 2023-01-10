use dockworker::{ContainerCreateOptions, Docker};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let mut create = ContainerCreateOptions::new("hello-world:linux");
    create.tty(true);
    let container = docker
        .create_container(Some("testing"), &create)
        .await
        .unwrap();
    docker.start_container(&container.id).await.unwrap();
}
