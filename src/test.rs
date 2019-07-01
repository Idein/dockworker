#![cfg(test)]

use super::container::HealthState;
use super::ImageLayer;
use container::{Container, ContainerInfo};
use filesystem::FilesystemChange;
use hyper_client::Response;
use image::{Image, SummaryImage};
use process::Top;
use serde_json;
use stats::{Stats, StatsReader};
use system::SystemInfo;
use version::Version;

#[test]
fn get_containers() {
    let response = get_containers_response();
    assert!(serde_json::from_str::<Vec<Container>>(&response).is_ok())
}

#[test]
fn get_stats_single() {
    let response = get_stats_single_event(1);
    print!("{}", response);
    assert!(serde_json::from_str::<Stats>(&response).is_ok())
}

#[test]
fn get_stats_streaming() {
    let response = get_stats_response();
    let mut reader = StatsReader::new(response);

    let stats = reader.next().unwrap().unwrap();
    assert_eq!(&stats.read, "2015-04-09T07:02:08.480022081Z");

    let stats = reader.next().unwrap().unwrap();
    assert_eq!(&stats.read, "2015-04-09T07:02:08.480022082Z");

    let stats = reader.next().unwrap().unwrap();
    assert_eq!(&stats.read, "2015-04-09T07:02:08.480022083Z");

    assert!(reader.next().is_none());
}

#[test]
fn get_system_info() {
    let response = get_system_info_response();
    assert!(serde_json::from_str::<SystemInfo>(&response).is_ok())
}

#[test]
fn get_image_list() {
    let response = get_image_list_response();
    let images: Vec<SummaryImage> = serde_json::from_str(&response).unwrap();
    assert_eq!(3, images.len());
}

#[test]
fn get_image() {
    let response = get_image_response();
    println!("response: {:?}", serde_json::from_str::<Image>(&response));
}

#[test]
fn get_image_history() {
    let response = get_image_history_reponse();
    let images: Vec<ImageLayer> = serde_json::from_str(&response).unwrap();
    assert_ne!(images[0].id, None);
    assert_eq!(2, images.len());
}

#[test]
fn get_container_info() {
    let response = get_container_info_response();
    assert!(serde_json::from_str::<ContainerInfo>(&response).is_ok());

    let response = get_container_info_response_with_healthcheck();
    assert!(serde_json::from_str::<ContainerInfo>(&response).is_ok());
}

#[test]
fn get_healthcheck_info() {
    let response = get_container_info_response_with_healthcheck();
    let container_info = serde_json::from_str::<ContainerInfo>(&response).unwrap();
    assert!(container_info.State.Health.is_some());
    assert!(container_info.State.Health.unwrap().Status == HealthState::Healthy);
}

#[test]
fn get_processes() {
    let response = get_processes_response();
    assert!(serde_json::from_str::<Top>(&response).is_ok())
}

#[test]
fn get_filesystem_changes() {
    let response = get_filesystem_changes_response();
    assert!(serde_json::from_str::<Vec<FilesystemChange>>(&response).is_ok())
}

#[test]
fn get_version() {
    let response = get_version_response();
    assert!(serde_json::from_str::<Version>(&response).is_ok())
}

fn get_containers_response() -> String {
    r#"
    [
      {
        "Id": "ed3221f4adc05b9ecfbf56b1aa76d4e6e70d5b73b3876c322fc10d017c64ca86",
        "Names": [
          "/rust"
        ],
        "Image": "ghmlee/rust:latest",
        "ImageID": "533da4fa223bfbca0f56f65724bb7a4aae7a1acd6afa2309f370463eaf9c34a4",
        "Command": "bash",
        "Created": 1439434052,
        "Ports": [
          {
            "IP": "0.0.0.0",
            "PrivatePort": 8888,
            "PublicPort": 8888,
            "Type": "tcp"
          }
        ],
        "SizeRootFs": 253602755,
        "Labels": {},
        "State": "exited",
        "Status": "Exited (137) 12 hours ago",
        "HostConfig": {
          "NetworkMode": "default"
        },
        "NetworkSettings": {
          "Networks": {
            "bridge": {
              "IPAMConfig": null,
              "Links": null,
              "Aliases": null,
              "NetworkID": "c033e08c176af51c8eca4aca77a0a6b3def00f181918ecd0836589d74e94973a",
              "EndpointID": "7b4f20e7a13f2ccbfc31f3252dc1ca3afb65b5eb2b7250fe93074c6e83671baf",
              "Gateway": "10.10.0.1",
              "IPAddress": "10.10.0.4",
              "IPPrefixLen": 24,
              "IPv6Gateway": "",
              "GlobalIPv6Address": "",
              "GlobalIPv6PrefixLen": 0,
              "MacAddress": "02:42:0a:0a:00:04",
              "DriverOpts": null
            },
            "none": {
              "IPAMConfig": null,
              "Links": null,
              "Aliases": null,
              "NetworkID": "3d8e6b21bced2737e634f897a54b83973da92498fe0774aa5fb6d8217b2c9322",
              "EndpointID": "4cd498f3bc50a9c3e9ae606d94d447b121bb2719701410d5cc98f6a033349ec1",
              "Gateway": "",
              "IPAddress": "",
              "IPPrefixLen": 0,
              "IPv6Gateway": "",
              "GlobalIPv6Address": "",
              "GlobalIPv6PrefixLen": 0,
              "MacAddress": "",
              "DriverOpts": null
            }
          }
        },
        "Mounts": [],
        "SizeRw": 10832473
      }
    ]
    "#
    .into()
}

fn get_system_info_response() -> String {
    "{\"Containers\":6,\"Debug\":0,\"DockerRootDir\":\"/var/lib/docker\",\"Driver\":\"btrfs\",\"DriverStatus\":[[\"Build Version\",\"Btrfs v3.17.1\"],[\"Library Version\",\"101\"]],\"ExecutionDriver\":\"native-0.2\",\"ID\":\"WG63:3NIU:TSI2:FV7J:IL2O:YPXA:JR3F:XEKT:JZVR:JA6T:QMYE:B4SB\",\"IPv4Forwarding\":1,\"Images\":190,\"IndexServerAddress\":\"https://index.docker.io/v1/\",\"InitPath\":\"/usr/libexec/docker/dockerinit\",\"InitSha1\":\"30c93967bdc3634b6036e1a76fd547bbe171b264\",\"KernelVersion\":\"3.18.6\",\"Labels\":null,\"MemTotal\":16854257664,\"MemoryLimit\":1,\"NCPU\":4,\"NEventsListener\":0,\"NFd\":68,\"NGoroutines\":95,\"Name\":\"core\",\"OperatingSystem\":\"CoreOS 607.0.0\",\"RegistryConfig\":{\"IndexConfigs\":{\"docker.io\":{\"Mirrors\":null,\"Name\":\"docker.io\",\"Official\":true,\"Secure\":true}},\"InsecureRegistryCIDRs\":[\"127.0.0.0/8\"]},\"SwapLimit\":1}".to_string()
}

// `docker inspect debian:wheely-2019- |  jq '.[]'
fn get_image_response() -> String {
    r###"{"Id":"sha256:301e280df919c411b7c2b049f938f3e26e4269a9be4a8ac3babce1ede930be0f","RepoTags":["debian:wheezy-20190204-slim"],"RepoDigests":["debian@sha256:8af4c5d36bf9e97bd9e9d32f4b23c30197269a8690d1aee6771beb7bdc744d5d"],"Parent":"","Comment":"","Created":"2019-02-06T03:31:46.89466512Z","Container":"bde93de20096d0e854d13ce9e3e8a506b3a2b7798c051bd1e2bebb644ad60b9a","ContainerConfig":{"Hostname":"bde93de20096","Domainname":"","User":"","AttachStdin":false,"AttachStdout":false,"AttachStderr":false,"Tty":false,"OpenStdin":false,"StdinOnce":false,"Env":["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],"Cmd":["/bin/sh","-c","#(nop) ","CMD [\"bash\"]"],"ArgsEscaped":true,"Image":"sha256:cf2bd8704a6c5c4fc4b7a9801c5dacac8ddd3fc699b6e02227119b168d8cf2a9","Volumes":null,"WorkingDir":"","Entrypoint":null,"OnBuild":null,"Labels":{}},"DockerVersion":"18.06.1-ce","Author":"","Config":{"Hostname":"","Domainname":"","User":"","AttachStdin":false,"AttachStdout":false,"AttachStderr":false,"Tty":false,"OpenStdin":false,"StdinOnce":false,"Env":["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],"Cmd":["bash"],"ArgsEscaped":true,"Image":"sha256:cf2bd8704a6c5c4fc4b7a9801c5dacac8ddd3fc699b6e02227119b168d8cf2a9","Volumes":null,"WorkingDir":"","Entrypoint":null,"OnBuild":null,"Labels":null},"Architecture":"amd64","Os":"linux","Size":46924746,"VirtualSize":46924746,"GraphDriver":{"Data":null,"Name":"aufs"},"RootFS":{"Type":"layers","Layers":["sha256:745d171eb8c3d69f788da3a1b053056231ad140b80be71d6869229846a1f3a77"]},"Metadata":{"LastTagTime":"0001-01-01T00:00:00Z"}}"###.into()
}

fn get_image_list_response() -> String {
    "[{\"Created\":1428533761,\"Id\":\"533da4fa223bfbca0f56f65724bb7a4aae7a1acd6afa2309f370463eaf9c34a4\",\"ParentId\":\"84ac0b87e42afe881d36f03dea817f46893f9443f9fc10b64ec279737384df12\",\"RepoTags\":[\"ghmlee/rust:nightly\"],\"Size\":0,\"VirtualSize\":806688288},{\"Created\":1371157430,\"Id\":\"511136ea3c5a64f264b78b5433614aec563103b4d4702f3ba7d4d2698e22c158\",\"ParentId\":\"\",\"RepoTags\":[],\"Size\":0,\"VirtualSize\":0},
    {\"Created\":1371157430,\"Id\":\"511136ea3c5a64f264b78b5433614aec563103b4d4702f3ba7d4d2698e22c158\",\"ParentId\":\"\",\"RepoTags\":null,\"Size\":0,\"VirtualSize\":0}]".to_string()
}

fn get_image_history_reponse() -> String {
    // First has Id, second has Id missing.
    r#"[{
            "Comment": "",
            "Created": 1539614714,
            "CreatedBy": "/bin/sh -c apk add --update openssl",
            "Id": "1234",
            "Size": 4736047,
            "Tags": null
        },
        {
            "Comment": "",
            "Created": 1536704390,
            "CreatedBy": "/bin/sh -c #(nop) ADD file:25c10b1d1b41d46a1827ad0b0d2389c24df6d31430005ff4e9a2d84ea23ebd42 in / ",
            "Id": "<missing>",
            "Size": 4413370,
            "Tags": null
        }
    ]
    "#.into()
}

fn get_container_info_response() -> &'static str {
    include_str!("fixtures/container_inspect.json")
}

fn get_container_info_response_with_healthcheck() -> &'static str {
    include_str!("fixtures/container_inspect_health.json")
}

fn get_processes_response() -> String {
    "{\"Processes\":[[\"4586\",\"999\",\"rust\"]],\"Titles\":[\"PID\",\"USER\",\"COMMAND\"]}"
        .to_string()
}

fn get_filesystem_changes_response() -> String {
    "[{\"Path\":\"/tmp\",\"Kind\":0}]".to_string()
}

fn get_version_response() -> String {
    "{\"Version\":\"1.8.1\",\"ApiVersion\":\"1.20\",\"GitCommit\":\"d12ea79\",\"GoVersion\":\"go1.4.2\",\"Os\":\"linux\",\"Arch\":\"amd64\",\"KernelVersion\":\"4.0.9-boot2docker\",\"BuildTime\":\"Thu Aug 13 02:49:29 UTC 2015\"}".to_string()
}

fn get_stats_response() -> Response {
    let mut response = http::Response::builder();
    response.status(http::StatusCode::OK);
    response.header("Transfer-Encoding", "chunked");
    response.header("Connection", "Close");
    let s1 = get_stats_single_event(1);
    let s2 = get_stats_single_event(2);
    let s3 = get_stats_single_event(3);
    Response::new(
        response
            .body(hyper::Body::from(format!("{}\n{}\n{}", s1, s2, s3)))
            .unwrap(),
    )
}

fn get_stats_single_event(n: u64) -> String {
    let template = include_str!("fixtures/stats_single_event.json")
        .to_string()
        .replace("\n", "");
    template.replace("{}", &n.to_string())
}
