use dockworker::{ContainerBuildOptions, Docker};
use futures::stream::StreamExt;
use std::path::Path;
use tar::Builder;

#[tokio::main]
async fn main() {
    {
        use tokio::io::AsyncWriteExt;
        let mut dockerfile = tokio::fs::File::create("Dockerfile").await.unwrap();
        dockerfile
            .write_all(
                r#"FROM alpine:edge
        RUN echo Hi mum
        "#
                .as_bytes(),
            )
            .await
            .unwrap();
    }
    // Create tar file
    {
        let tar_file = tokio::fs::File::create("image.tar")
            .await
            .unwrap()
            .into_std()
            .await;
        let mut a = Builder::new(tar_file);
        a.append_path("Dockerfile").unwrap();
    }

    let docker = Docker::connect_with_defaults().unwrap();
    let name = "test-image";
    let tag = "latest";
    println!("build an image {name}:{tag} ...");
    let options = ContainerBuildOptions {
        dockerfile: "Dockerfile".into(),
        t: vec!["silly:lat".to_owned()],
        ..ContainerBuildOptions::default()
    };

    let mut stream = docker
        .build_image(options, Path::new("image.tar"))
        .await
        .unwrap();
    while let Some(msg) = stream.next().await {
        println!("msg: {:?}", msg);
    }
}
