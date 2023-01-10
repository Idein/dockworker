extern crate dockworker;
extern crate env_logger;

use dockworker::Docker;
use std::path::Path;

fn main() {
    env_logger::init();
    let docker = Docker::connect_with_defaults().unwrap();
    let id = docker
        .load_image(false, Path::new("image.tar"))
        .expect("prepare a tar-archive like: $docker save busybox > image.tar");
    println!("loaded: {id}");
}
