use dockworker::Docker;

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    println!("{:#?}", docker.version().await.unwrap());
}
