use dockworker::Docker;

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();

    let name = "debian";
    let tag = "latest";
    println!("create an image {name}:{tag} ...");
    let mut stats = docker.create_image(name, tag).await.unwrap();
    use futures::stream::StreamExt;
    while let Some(stat) = stats.next().await {
        println!("{stat:?}");
    }
}
