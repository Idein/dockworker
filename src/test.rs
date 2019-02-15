#![cfg(test)]

use super::ImageLayer;
use container::{Container, ContainerInfo};
use filesystem::FilesystemChange;
use hyper_client::Response;
use image::{SummaryImage, Image};
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
    assert!(serde_json::from_str::<Image>(&response).is_ok());
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
    assert!(serde_json::from_str::<ContainerInfo>(&response).is_ok())
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
    "[{\"Id\":\"ed3221f4adc05b9ecfbf56b1aa76d4e6e70d5b73b3876c322fc10d017c64ca86\",\"Names\":[\"/rust\"],\"Image\":\"ghmlee/rust:latest\",\"Command\":\"bash\",\"Created\":1439434052,\"Ports\":[{\"IP\":\"0.0.0.0\",\"PrivatePort\":8888,\"PublicPort\":8888,\"Type\":\"tcp\"}],\"SizeRootFs\":253602755,\"Labels\":{},\"Status\":\"Exited (137) 12 hours ago\",\"HostConfig\":{\"NetworkMode\":\"default\"},\"SizeRw\":10832473}]".to_string()
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

fn get_container_info_response() -> String {
    r#"{"Id":"774758ca1db8d05bd848d2b3456c8253a417a0511329692869df1cbe82978d37","Created":"2016-10-25T11:59:37.858589354Z","Path":"rails","Args":["server","-b","0.0.0.0"],"State":{"Status":"running","Running":true,"Paused":false,"Restarting":false,"OOMKilled":false,"Dead":false,"Pid":13038,"ExitCode":0,"Error":"","StartedAt":"2016-10-25T11:59:38.261828009Z","FinishedAt":"0001-01-01T00:00:00Z"},"Image":"sha256:f5e9d349e7e5c0f6de798d732d83fa5e087695cd100149121f01c891e6167c13","ResolvConfPath":"/var/lib/docker/containers/774758ca1db8d05bd848d2b3456c8253a417a0511329692869df1cbe82978d37/resolv.conf","HostnamePath":"/var/lib/docker/containers/774758ca1db8d05bd848d2b3456c8253a417a0511329692869df1cbe82978d37/hostname","HostsPath":"/var/lib/docker/containers/774758ca1db8d05bd848d2b3456c8253a417a0511329692869df1cbe82978d37/hosts","LogPath":"/var/lib/docker/containers/774758ca1db8d05bd848d2b3456c8253a417a0511329692869df1cbe82978d37/774758ca1db8d05bd848d2b3456c8253a417a0511329692869df1cbe82978d37-json.log","Name":"/railshello_web_1","RestartCount":0,"Driver":"aufs","MountLabel":"","ProcessLabel":"","AppArmorProfile":"","ExecIDs":null,"HostConfig":{"Binds":[],"ContainerIDFile":"","LogConfig":{"Type":"json-file","Config":{}},"NetworkMode":"railshello_default","PortBindings":{"3000/tcp":[{"HostIp":"","HostPort":"3000"}]},"RestartPolicy":{"Name":"","MaximumRetryCount":0},"AutoRemove":false,"VolumeDriver":"","VolumesFrom":[],"CapAdd":null,"CapDrop":null,"Dns":null,"DnsOptions":null,"DnsSearch":null,"ExtraHosts":null,"GroupAdd":null,"IpcMode":"","Cgroup":"","Links":null,"OomScoreAdj":0,"PidMode":"","Privileged":false,"PublishAllPorts":false,"ReadonlyRootfs":false,"SecurityOpt":null,"UTSMode":"","UsernsMode":"","ShmSize":67108864,"Runtime":"runc","ConsoleSize":[0,0],"Isolation":"","CpuShares":0,"Memory":0,"CgroupParent":"","BlkioWeight":0,"BlkioWeightDevice":null,"BlkioDeviceReadBps":null,"BlkioDeviceWriteBps":null,"BlkioDeviceReadIOps":null,"BlkioDeviceWriteIOps":null,"CpuPeriod":0,"CpuQuota":0,"CpusetCpus":"","CpusetMems":"","Devices":null,"DiskQuota":0,"KernelMemory":0,"MemoryReservation":0,"MemorySwap":0,"MemorySwappiness":-1,"OomKillDisable":false,"PidsLimit":0,"Ulimits":null,"CpuCount":0,"CpuPercent":0,"IOMaximumIOps":0,"IOMaximumBandwidth":0},"GraphDriver":{"Name":"aufs","Data":null},"Mounts":[],"Config":{"Hostname":"774758ca1db8","Domainname":"","User":"","AttachStdin":false,"AttachStdout":false,"AttachStderr":false,"ExposedPorts":{"3000/tcp":{}},"Tty":false,"OpenStdin":false,"StdinOnce":false,"Env":["RACK_ENV=development","PROJECT_NAME=rails_hello","GLOBAL_PASSWORD=magic","SOME_PASSWORD=secret","RAILS_ENV=development","DATABASE_URL=postgres://postgres@db:5432/rails_hello_development","PATH=/usr/local/bundle/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin","RUBY_MAJOR=2.3","RUBY_VERSION=2.3.1","RUBY_DOWNLOAD_SHA256=b87c738cb2032bf4920fef8e3864dc5cf8eae9d89d8d523ce0236945c5797dcd","RUBYGEMS_VERSION=2.6.7","BUNDLER_VERSION=1.13.4","GEM_HOME=/usr/local/bundle","BUNDLE_PATH=/usr/local/bundle","BUNDLE_BIN=/usr/local/bundle/bin","BUNDLE_SILENCE_ROOT_WARNING=1","BUNDLE_APP_CONFIG=/usr/local/bundle"],"Cmd":["rails","server","-b","0.0.0.0"],"Image":"faraday/rails_hello","Volumes":null,"WorkingDir":"/usr/src/app","Entrypoint":null,"OnBuild":null,"Labels":{"com.docker.compose.config-hash":"ff040c76ba24b1bac8d89e95cfb5ba7e29bd19423ed548a1436ae3c94bc6381a","com.docker.compose.container-number":"1","com.docker.compose.oneoff":"False","com.docker.compose.project":"railshello","com.docker.compose.service":"web","com.docker.compose.version":"1.8.1","io.fdy.cage.lib.coffee_rails":"/usr/src/app/vendor/coffee-rails","io.fdy.cage.pod":"frontend","io.fdy.cage.shell":"bash","io.fdy.cage.srcdir":"/usr/src/app","io.fdy.cage.target":"development","io.fdy.cage.test":"bundle exec rake"}},"NetworkSettings":{"Bridge":"","SandboxID":"ca243185e052f364f6f9e4141ee985397cda9c66a87258f8a8048a05452738cf","HairpinMode":false,"LinkLocalIPv6Address":"","LinkLocalIPv6PrefixLen":0,"Ports":{"3000/tcp":[{"HostIp":"0.0.0.0","HostPort":"3000"}]},"SandboxKey":"/var/run/docker/netns/ca243185e052","SecondaryIPAddresses":null,"SecondaryIPv6Addresses":null,"EndpointID":"","Gateway":"","GlobalIPv6Address":"","GlobalIPv6PrefixLen":0,"IPAddress":"","IPPrefixLen":0,"IPv6Gateway":"","MacAddress":"","Networks":{"railshello_default":{"IPAMConfig":null,"Links":null,"Aliases":["web","774758ca1db8"],"NetworkID":"4b237b1de0928a11bb399adaa249705b666bdc5dece3e9bdc260a630643bf945","EndpointID":"7d5e1e9df4bdf400654b96afdd1d42040c150a4f5b414f084c8fd5c95a9a906e","Gateway":"172.24.0.1","IPAddress":"172.24.0.3","IPPrefixLen":16,"IPv6Gateway":"","GlobalIPv6Address":"","GlobalIPv6PrefixLen":0,"MacAddress":"02:42:ac:18:00:03"}}}}"#.to_string()
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
    format!("{{\"read\":\"2015-04-09T07:02:08.48002208{}Z\",\"network\":{{\"rx_bytes\":5820720,\"rx_packets\":2742,\"rx_errors\":0,\"rx_dropped\":1,\"tx_bytes\":158527,\"tx_packets\":2124,\"tx_errors\":0,\"tx_dropped\":0}},\"cpu_stats\":{{\"cpu_usage\":{{\"total_usage\":19194125000,\"percpu_usage\":[14110113138,3245604417,845722573,992684872],\"usage_in_kernelmode\":1110000000,\"usage_in_usermode\":18160000000}},\"system_cpu_usage\":1014488290000000,\"throttling_data\":{{\"periods\":0,\"throttled_periods\":0,\"throttled_time\":0}}}},\"memory_stats\":{{\"usage\":208437248,\"max_usage\":318791680,\"stats\":{{\"active_anon\":27213824,\"active_file\":129069056,\"cache\":178946048,\"hierarchical_memory_limit\":18446744073709551615,\"hierarchical_memsw_limit\":18446744073709551615,\"inactive_anon\":0,\"inactive_file\":49876992,\"mapped_file\":10809344,\"pgfault\":99588,\"pgmajfault\":819,\"pgpgin\":130731,\"pgpgout\":153466,\"rss\":29331456,\"rss_huge\":6291456,\"swap\":0,\"total_active_anon\":27213824,\"total_active_file\":129069056,\"total_cache\":178946048,\"total_inactive_anon\":0,\"total_inactive_file\":49876992,\"total_mapped_file\":10809344,\"total_pgfault\":99588,\"total_pgmajfault\":819,\"total_pgpgin\":130731,\"total_pgpgout\":153466,\"total_rss\":29331456,\"total_rss_huge\":6291456,\"total_swap\":0,\"total_unevictable\":0,\"total_writeback\":0,\"unevictable\":0,\"writeback\":0}},\"failcnt\":0,\"limit\":16854257664}},\"blkio_stats\":{{\"io_service_bytes_recursive\":[{{\"major\":8,\"minor\":0,\"op\":\"Read\",\"value\":150687744}},{{\"major\":8,\"minor\":0,\"op\":\"Write\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Sync\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Async\",\"value\":150687744}},{{\"major\":8,\"minor\":0,\"op\":\"Total\",\"value\":150687744}}],\"io_serviced_recursive\":[{{\"major\":8,\"minor\":0,\"op\":\"Read\",\"value\":484}},{{\"major\":8,\"minor\":0,\"op\":\"Write\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Sync\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Async\",\"value\":484}},{{\"major\":8,\"minor\":0,\"op\":\"Total\",\"value\":484}}],\"io_queue_recursive\":[{{\"major\":8,\"minor\":0,\"op\":\"Read\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Write\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Sync\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Async\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Total\",\"value\":0}}],\"io_service_time_recursive\":[{{\"major\":8,\"minor\":0,\"op\":\"Read\",\"value\":2060941295}},{{\"major\":8,\"minor\":0,\"op\":\"Write\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Sync\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Async\",\"value\":2060941295}},{{\"major\":8,\"minor\":0,\"op\":\"Total\",\"value\":2060941295}}],\"io_wait_time_recursive\":[{{\"major\":8,\"minor\":0,\"op\":\"Read\",\"value\":5476872825}},{{\"major\":8,\"minor\":0,\"op\":\"Write\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Sync\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Async\",\"value\":5476872825}},{{\"major\":8,\"minor\":0,\"op\":\"Total\",\"value\":5476872825}}],\"io_merged_recursive\":[{{\"major\":8,\"minor\":0,\"op\":\"Read\",\"value\":79}},{{\"major\":8,\"minor\":0,\"op\":\"Write\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Sync\",\"value\":0}},{{\"major\":8,\"minor\":0,\"op\":\"Async\",\"value\":79}},{{\"major\":8,\"minor\":0,\"op\":\"Total\",\"value\":79}}],\"io_time_recursive\":[{{\"major\":8,\"minor\":0,\"op\":\"\",\"value\":1814}}],\"sectors_recursive\":[{{\"major\":8,\"minor\":0,\"op\":\"\",\"value\":294312}}]}}}}", n).to_string()
}
