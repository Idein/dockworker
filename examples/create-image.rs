extern crate boondock;

use boondock::Docker;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let statuses = docker.create_image("debian", "latest").unwrap();

    for status in statuses {
        println!("{:?}", status);
    }
}
