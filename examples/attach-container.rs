extern crate boondock;
extern crate byteorder;
extern crate hyper;

use std::io::Read;
use std::str;
use boondock::{ContainerCreateOptions, Docker};
use hyper::status::StatusCode;
use byteorder::{BigEndian, ReadBytesExt};

fn main() {
    let docker = Docker::connect_with_defaults().unwrap();
    let create = ContainerCreateOptions::new("hello-world:linux");

    let container = docker.create_container("testing", &create).unwrap();
    docker.start_container(&container.id).unwrap();
    let mut res = docker
        .attach_container(&container.id, None, true, true, false, true, false)
        .unwrap();

    if res.status == StatusCode::Ok {
        let mut buf = [0u8; 8];
        while res.read_exact(&mut buf).is_ok() {
            // read 8 bytes
            assert_eq!(buf[0], 1); // stdout

            let mut frame_size_raw = &buf[4..];
            let frame_size = frame_size_raw.read_u32::<BigEndian>().unwrap();

            let mut frame = vec![0; frame_size as usize];
            res.read_exact(&mut frame).unwrap();

            print!("frame:{}", str::from_utf8(&frame).unwrap());
        }
    }
}
