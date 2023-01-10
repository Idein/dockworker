extern crate dockworker;
extern crate hyper;

use dockworker::Docker;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let token = docker
        .auth(
            "someusername",
            "somepassword",
            "someusername@example.com",
            "localhost:5000",
        )
        .unwrap();
    println!("token: {token:?}");
}
