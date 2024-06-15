use dockworker::{container::ContainerFilters, Docker};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let filter = ContainerFilters::new();
    let containers = docker
        .list_containers(None, None, None, filter)
        .await
        .unwrap();

    containers.iter().for_each(|c| {
        println!("{c:?}");
    });
}
