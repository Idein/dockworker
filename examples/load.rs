use dockworker::Docker;
use std::path::Path;

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let id = docker
        .load_image(false, Path::new("image.tar"))
        .await
        .expect("prepare a tar-archive like: $docker save busybox > image.tar");
    println!("loaded: {id}");
}
