extern crate dockworker;
extern crate failure;

use dockworker::container::ContainerFilters;
use dockworker::errors::*;
use dockworker::Docker;
use failure::Fail;

fn find_all_exported_ports() -> Result<()> {
    let docker = Docker::connect_with_defaults()?;
    let containers = docker.list_containers(Some(true), None, None, ContainerFilters::default())?;
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
        for e in Fail::iter_causes(&err) {
            eprintln!("{}", e);
        }
    }
}
