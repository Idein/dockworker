use dockworker::{container::ContainerFilters, Docker};

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let mut file = tokio::fs::File::create("temp.tar").await.unwrap();
    if let Some(container) = docker
        .list_containers(None, None, None, ContainerFilters::default())
        .await
        .unwrap()
        .first()
    {
        let res = docker
            .export_container(container.Id.as_str())
            .await
            .unwrap();
        use futures::stream::TryStreamExt;
        let mut res = tokio_util::io::StreamReader::new(res.map_err(std::io::Error::other));
        tokio::io::copy(&mut res, &mut file).await.unwrap();
    }
}
