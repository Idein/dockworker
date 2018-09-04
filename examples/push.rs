extern crate dockworker;
extern crate hyper;

use dockworker::Docker;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let (name, tag) = ("alpine", "latest");
    docker
        .create_image(name, tag)
        .unwrap()
        .for_each(|_| print!("."));
    let serveraddress = "localhost:5000";
    println!("pulled: {}:{}", name, tag);
    docker
        .push_image(
            &format!("{}/{}", serveraddress, name),
            tag,
            "someusername",
            "somepassword",
            "someusername@example.com",
            serveraddress,
        )
        .unwrap();
    println!("pushed: {}:{}", name, tag);
}
