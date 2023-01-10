use dockworker::{ContainerCreateOptions, Docker};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let mut events = docker.events(None, None, None).await.unwrap();

    let create = ContainerCreateOptions::new("hello-world:linux");
    docker.create_image("hello-world", "linux").await.unwrap();
    let container = docker.create_container(None, &create).await.unwrap();
    docker.start_container(&container.id).await.unwrap();

    use futures::stream::StreamExt;
    while let Some(e) = events.next().await {
        let e = e.unwrap();
        if e.Type == "network" && e.Action == "disconnect" {
            println!("{e:?}");
        }
    }

    docker
        .remove_container(&container.id, None, Some(true), None)
        .await
        .unwrap();
}
