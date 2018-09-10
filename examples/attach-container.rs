extern crate dockworker;
extern crate hyper;

use dockworker::{ContainerCreateOptions, ContainerHostConfig, Docker,
                 container::AttachResponseStreamReader};
use std::str;
use std::io::{BufRead, BufReader, Read};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let mut host_config = ContainerHostConfig::new();
    host_config.auto_remove(true);
    let mut create = ContainerCreateOptions::new("hello-world:linux");
    create.host_config(host_config);

    let container = docker.create_container(Some("testing"), &create).unwrap();
    docker.start_container(&container.id).unwrap();
    let stream = docker
        .attach_container(&container.id, None, true, true, false, true, false)
        .unwrap();
    let reader: AttachResponseStreamReader = stream.into();
    let mut line_reader = BufReader::new(reader);

    loop {
        let mut line = String::new();
        let size = line_reader.read_line(&mut line).unwrap();
        print!("{}: {}", size, line);
        if size == 0 {
            break;
        }
    }
    print!("");

    /* // accumurate whole output
    let mut whole = Vec::new();
    println!("start");
    loop {
        let mut buf: [u8; 7] = [0u8; 7];
        let x = reader.read(&mut buf).unwrap();
        println!("read: {}", x);
        if x == 0 {
            println!("end of stream");
            break;
        }
        println!("copy");
        whole.extend_from_slice(&buf);
    }
    println!("all: {}", String::from_utf8_lossy(&whole));
    */
    /*
    for frame in iter {
        match frame {
            Ok(frame) => print!("frame:{}", str::from_utf8(frame.as_bytes()).unwrap()),
            Err(err) => println!("frame:err:{:?}", err),
        }
    }
    */
}
