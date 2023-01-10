use dockworker::{ContainerCreateOptions, Docker};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let create = ContainerCreateOptions::new("hello-world:linux");
    let container = docker
        .create_container(Some("testing"), &create)
        .await
        .unwrap();
    println!("{container:?}")
}
