use dockworker::{container::ContainerFilters, Docker};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();

    let prunedt = docker.prune_image(true).await.unwrap();
    println!("pruned(true): {prunedt:?}");

    let prunedf = docker.prune_image(false).await.unwrap();
    println!("pruned(false): {prunedf:?}");

    let containers = docker
        .list_containers(Some(true), None, None, ContainerFilters::new())
        .await
        .unwrap();

    containers.iter().for_each(|c| {
        println!("image: {}", c.Image);
    });
}
