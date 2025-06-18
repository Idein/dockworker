use dockworker::{ContainerPruneFilters, Docker};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();

    // Prune all stopped containers without filters
    let result = docker
        .prune_containers(ContainerPruneFilters::new())
        .await
        .unwrap();
    println!("Prune result (no filters): {result:?}");

    // Prune containers with label filter
    let mut filters = ContainerPruneFilters::new();
    filters.label("test=example".to_string());

    let result = docker.prune_containers(filters).await.unwrap();
    println!("Prune result (with label filter): {result:?}");

    // Prune containers older than 24 hours
    let mut filters = ContainerPruneFilters::new();
    filters.until("24h".to_string());

    let result = docker.prune_containers(filters).await.unwrap();
    println!("Prune result (until 24h): {result:?}");
}
