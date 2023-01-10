use dockworker::Docker;

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let histories = docker.history_image("my_image_name").await.unwrap();

    for change in histories {
        println!("{change:#?}");
    }
}
