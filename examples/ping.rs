extern crate dockworker;

use dockworker::Docker;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    docker.ping().unwrap();
}
