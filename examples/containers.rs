extern crate dockworker;

use dockworker::{Docker, container::ContainerFilters};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();

    let info0 = docker.container_info("container").unwrap();
    let info1 = docker.container_info("container_with_tty").unwrap();
    println!("info0.Config.Tty: {}", info0.Config.Tty);
    println!("info1.Config.Tty: {}", info1.Config.Tty);
}
