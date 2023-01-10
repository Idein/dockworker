use dockworker::{container::ContainerFilters, Docker};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    if let Some(container) = docker
        .list_containers(None, None, None, ContainerFilters::default())
        .await
        .unwrap()
        .get(0)
    {
        let processes = docker.processes(container.Id.as_str()).await.unwrap();
        for process in processes {
            println!("{process:#?}");
        }
    }
}
