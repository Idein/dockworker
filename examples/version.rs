extern crate dockworker;

use dockworker::Docker;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    println!("{:#?}", docker.version().unwrap());
}
