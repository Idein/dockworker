extern crate dockworker;

use dockworker::Docker;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let history = docker.history_image("my_image_name");

    println!("History:");
    match history {
        Ok(changes) => {
            for change in changes {
                println!("{change:#?}");
            }
        }
        Err(e) => {
            println!("Error {e}");
        }
    }
}
