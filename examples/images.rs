use dockworker::Docker;

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let images = docker.images(false).await.unwrap();

    for image in &images {
        println!(
            "{} -> Size: {}, Virtual Size: {}, Created: {}",
            image.Id, image.Size, image.VirtualSize, image.Created
        );
    }
}
