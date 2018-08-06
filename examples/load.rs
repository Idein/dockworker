extern crate boondock;

use std::path::Path;
use boondock::Docker;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    docker
        .load_image(false, Path::new("image.tar"))
        .expect("prepare a tar-archive like: $docker save busybox > image.tar");
}
