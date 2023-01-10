use dockworker::{container::ContainerFilters, Docker};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let containers = docker
        .list_containers(None, None, None, ContainerFilters::default())
        .await
        .unwrap();
    for container in containers {
        let mut stats = docker
            .stats(&container.Id, Some(false), Some(true))
            .await
            .unwrap();
        use futures::stream::StreamExt;
        while let Some(stats) = stats.next().await {
            println!("{:#?}", stats.unwrap());
        }
    }
}
