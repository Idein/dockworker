extern crate dockworker;

use dockworker::Docker;
use std::io::Write;
use std::io::stdout;

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let events = docker.events(None, None, None).unwrap();
    for event in events {
        println!("{:?}", event);
        stdout().flush();
    }
}
