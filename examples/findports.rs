use dockworker::{container::ContainerFilters, Docker};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let containers = docker
        .list_containers(Some(true), None, None, ContainerFilters::default())
        .await
        .unwrap();
    for container in &containers {
        let info = docker.container_info(container.Id.as_str()).await.unwrap();

        // Uncomment this to dump everything we know about a container.
        //println!("{:#?}", &info);

        println!("{}", info.Name);
        for (k, v) in info.NetworkSettings.Ports.iter() {
            println!("{k}: {v:?}");
        }
    }
}
