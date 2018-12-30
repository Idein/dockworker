extern crate dockworker;
extern crate tar;

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::fs::File;
use tar::Builder;
use dockworker::{ContainerBuildOptions, Docker};

fn main() {
    {
        let mut dockerfile = File::create("Dockerfile").unwrap();
        dockerfile
            .write_all(
                r#"FROM alpine:edge
        RUN echo Hi mum
        "#.as_bytes(),
            )
            .unwrap();
    }
    // Create tar file
    {
        let tar_file = File::create("image.tar").unwrap();
        let mut a = Builder::new(tar_file);
        a.append_path("Dockerfile").unwrap();
    }

    let docker = Docker::connect_with_defaults().unwrap();
    let name = "test-image";
    let tag = "latest";
    println!("build an image {}:{} ...", name, tag);
    let options = ContainerBuildOptions {
        dockerfile: "Dockerfile".into(),
        t: vec!["silly:lat".to_owned()],
        ..ContainerBuildOptions::default()
    };
    let res = docker.build_image(options, Path::new("image.tar")).unwrap();

    // read and discard to end of response
    for line in BufReader::new(res).lines() {
        let buf = line.unwrap();
        println!("{}", &buf);
    }
}
