extern crate dockworker;

use anyhow::Error;
use dockworker::errors::*;
use dockworker::{ContainerListOptions, Docker};

fn find_all_exported_ports() -> Result<()> {
    let docker = Docker::connect_with_defaults()?;
    let containers = docker.containers(ContainerListOptions::default().all())?;
    for container in &containers {
        let info = docker.container_info(container.Id.as_str())?;

        // Uncomment this to dump everything we know about a container.
        //println!("{:#?}", &info);

        println!("{}", info.Name);
        for (k, v) in info.NetworkSettings.Ports.iter() {
            println!("{}: {:?}", k, v);
        }
    }
    Ok(())
}

fn main() {
    if let Err(err) = find_all_exported_ports() {
        eprint!("Error: ");
        for e in Error::new(err).chain() {
            eprintln!("{}", e);
        }
    }
}
