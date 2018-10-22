//! Options which can be passed to various `Docker` commands.

use std::path::PathBuf;
use std::time::Duration;

use std::collections::HashMap;
use url::form_urlencoded;

use serde::de::{DeserializeOwned, Deserializer};
use serde::Deserialize;
use serde::ser::{Serialize, Serializer, SerializeStruct};

fn null_to_default<'de, D, T>(de: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    let actual: Option<T> = Option::deserialize(de)?;
    Ok(actual.unwrap_or_default())
}

/// Options for `Docker::containers`.  This uses a "builder" pattern, so
/// most methods will consume the object and return a new one.
#[derive(Debug, Clone, Default)]
pub struct ContainerListOptions {
    all: bool,
    //before: Option<String>,
    //filter: Filter,
    latest: bool,
    limit: Option<u64>,
    //since: Option<String>,
    size: bool,
}

impl ContainerListOptions {
    /// Return all containers, including stopped ones.
    pub fn all(mut self) -> Self {
        self.all = true;
        self
    }

    /// Return just the most-recently-started container (even if it has
    /// stopped).
    pub fn latest(mut self) -> Self {
        self.latest = true;
        self
    }

    /// Limit the number of containers we return.
    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    /// Calculate the total file sizes for our containers.  **WARNING:**
    /// This is very expensive.
    pub fn size(mut self) -> Self {
        self.size = true;
        self
    }

    /// Convert to URL parameters.
    pub fn to_url_params(&self) -> String {
        let mut params = form_urlencoded::Serializer::new(String::new());
        if self.all {
            params.append_pair("all", "1");
        }
        if self.latest {
            params.append_pair("latest", "1");
        }
        if let Some(limit) = self.limit {
            params.append_pair("limit", &limit.to_string());
        }
        if self.size {
            params.append_pair("size", "1");
        }
        params.finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct RestartPolicyImpl {
    Name: String,
    MaximumRetryCount: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct RestartPolicy(pub RestartPolicyImpl);

impl Default for RestartPolicy {
    fn default() -> Self {
        Self { None }
    }
}

impl Serialize for RestartPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            None => serializer.serialize_unit(),
            Some(RestartPolicyImpl {
                Name,
                MaximumRetryCount,
            }) => {
                let mut state = serializer.serialize_struct("RestartPolicy", 2)?;
                state.serialize_field("Name", &self.Name)?;
                state.serialize_field("MaximumRetryCount", &self.MaximumRetryCount)?;
                state.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_serialize_restart_policy() {
        let policy_some = RestartPolicy { Some(RestartPolicyImpl { Name: "", MaximumRetryCount: 0 }) };
        println!("policy: {}", serde_json::to_str(&policy_some).unwrap());
        let policy_none = RestartPolicy::default();
        println!("policy: {}", serde_json::to_str(&policy_none).unwrap());
    }
}


#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct DeviceMapping {
    PathOnHost: PathBuf,
    PathInContainer: PathBuf,
    /// combination of r,w,m
    CgroupPermissions: String,
}

impl DeviceMapping {
    pub fn new(
        path_on_host: PathBuf,
        path_in_container: PathBuf,
        cgroup_permissions: String,
    ) -> Self {
        Self {
            PathOnHost: path_on_host,
            PathInContainer: path_in_container,
            CgroupPermissions: cgroup_permissions,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerHostConfig {
    binds: Vec<String>,
    tmpfs: HashMap<String, String>,
    links: Vec<String>,
    memory: u64,
    memory_swap: u64,
    memory_reservation: u64,
    kernel_memory: u64,
    cpu_percent: u64, // TODO: Not sure what this should actually be
    // it could either be 0 - 100, or it could be 100% per thread.
    cpu_shares: u64,           // TODO: I don't have a single clue what this even is
    cpu_period: u64,           // TODO: Still no clue
    cpu_quota: u64,            // TODO: Still no clue
    cpuset_cpus: String,       // TODO: Still no clue
    io_maximum_bandwidth: u64, // TODO: Still no clue
    io_maximum_ops: u64,       // TODO: Still no clue
    blkio_weight: u64,         // TODO: Still no clue
    // blkio_weight_device: not sure the type of this. the provided is [{}]
    // blkio_weight_read_bps: not sure the type of this. the provided is [{}],
    // blkio_weight_read_iops: not sure the type of this. the provided is [{}]
    // blkio_weight_write_bps: not sure the type of this. the provided is [{}]
    // blkio_weight_write_iops: not sure the type of this. the provided is [{}]
    memory_swappiness: i32, // TODO: Maybe this can be smaller?
    oom_kill_disable: bool,
    oom_score_adj: u16, // TODO: Maybe this can be smaller?
    pid_mode: String,
    pids_limit: i16,
    port_bindings: HashMap<String, Vec<HashMap<String, String>>>,
    publish_all_ports: bool,
    privileged: bool,
    readonly_rootfs: bool,
    dns: Vec<String>,
    dns_options: Vec<String>,
    dns_search: Vec<String>,
    // extra_hosts: Not sure the type of this. the provided is null
    auto_remove: bool,
    volumes_from: Vec<String>,
    cap_add: Vec<String>,
    cap_drop: Vec<String>,
    group_add: Vec<String>,
    restart_policy: RestartPolicy,
    network_mode: String,
    devices: Vec<DeviceMapping>,
    sysctls: HashMap<String, String>,
    // ulimits: TODO: Not sure the type of this. the provided is [{}]
    // log_config: TODO: not sure the type of this. the provided makes no sense
    // security_opt: TODO: Not sure the type of this. The provided is []
    // storage_opt: TODO: Not sure the type of this. the provided is {}
    cgroup_parent: String,
    volume_driver: String,
    shm_size: u64,
}

impl ContainerHostConfig {
    pub fn new() -> Self {
        Self {
            binds: Vec::new(),
            tmpfs: HashMap::new(),
            links: Vec::new(),
            memory: 0,
            memory_swap: 0,
            memory_reservation: 0,
            kernel_memory: 0,
            cpu_percent: 0,
            cpu_shares: 0,
            cpu_period: 0,
            cpu_quota: 0,
            cpuset_cpus: "".to_owned(),
            io_maximum_bandwidth: 0,
            io_maximum_ops: 0,
            blkio_weight: 0,
            memory_swappiness: -1,
            oom_kill_disable: false,
            oom_score_adj: 0,
            pid_mode: "".to_owned(),
            pids_limit: 0,
            port_bindings: HashMap::new(),
            publish_all_ports: false,
            privileged: false,
            readonly_rootfs: false,
            dns: Vec::new(),
            dns_options: Vec::new(),
            dns_search: Vec::new(),
            auto_remove: false,
            volumes_from: Vec::new(),
            cap_add: Vec::new(),
            cap_drop: Vec::new(),
            group_add: Vec::new(),
            restart_policy: RestartPolicy::default(),
            network_mode: "default".to_owned(),
            devices: Vec::new(),
            sysctls: HashMap::new(),
            cgroup_parent: "".to_owned(),
            volume_driver: "".to_owned(),
            /// 64MB
            shm_size: 64 * 1024 * 1024,
        }
    }

    pub fn binds(&mut self, bind: String) -> &mut Self {
        self.binds.push(bind);
        self
    }
    pub fn tmpfs(&mut self, path: &str, option: &str) -> &mut Self {
        self.tmpfs.insert(path.to_owned(), option.to_owned());
        self
    }
    pub fn links(&mut self, link: String) -> &mut Self {
        self.links.push(link);
        self
    }
    pub fn memory(&mut self, memory: u64) -> &mut Self {
        self.memory = memory;
        self
    }
    pub fn memory_swap(&mut self, memory_swap: u64) -> &mut Self {
        self.memory_swap = memory_swap;
        self
    }
    pub fn memory_reservation(&mut self, memory_reservation: u64) -> &mut Self {
        self.memory_reservation = memory_reservation;
        self
    }
    pub fn kernel_memory(&mut self, kernel_memory: u64) -> &mut Self {
        self.kernel_memory = kernel_memory;
        self
    }
    pub fn cpu_percent(&mut self, cpu_percent: u64) -> &mut Self {
        self.cpu_percent = cpu_percent;
        self
    }
    pub fn cpu_shares(&mut self, cpu_shares: u64) -> &mut Self {
        self.cpu_shares = cpu_shares;
        self
    }
    pub fn cpu_period(&mut self, cpu_period: u64) -> &mut Self {
        self.cpu_period = cpu_period;
        self
    }
    pub fn cpu_quota(&mut self, cpu_quota: u64) -> &mut Self {
        self.cpu_quota = cpu_quota;
        self
    }
    pub fn cpuset_cpus(&mut self, cpuset_cpus: String) -> &mut Self {
        self.cpuset_cpus = cpuset_cpus;
        self
    }
    pub fn io_maximum_bandwidth(&mut self, io_maximum_bandwidth: u64) -> &mut Self {
        self.io_maximum_bandwidth = io_maximum_bandwidth;
        self
    }
    pub fn io_maximum_ops(&mut self, io_maximum_ops: u64) -> &mut Self {
        self.io_maximum_ops = io_maximum_ops;
        self
    }
    pub fn blkio_weight(&mut self, blkio_weight: u64) -> &mut Self {
        self.blkio_weight = blkio_weight;
        self
    }
    pub fn memory_swappiness(&mut self, memory_swappiness: i32) -> &mut Self {
        self.memory_swappiness = memory_swappiness;
        self
    }
    pub fn oom_kill_disable(&mut self, oom_kill_disable: bool) -> &mut Self {
        self.oom_kill_disable = oom_kill_disable;
        self
    }
    pub fn oom_score_adj(&mut self, oom_score_adj: u16) -> &mut Self {
        self.oom_score_adj = oom_score_adj;
        self
    }
    pub fn pid_mode(&mut self, pid_mode: String) -> &mut Self {
        self.pid_mode = pid_mode;
        self
    }
    pub fn pids_limit(&mut self, pids_limit: i16) -> &mut Self {
        self.pids_limit = pids_limit;
        self
    }
    pub fn publish_all_ports(&mut self, publish_all_ports: bool) -> &mut Self {
        self.publish_all_ports = publish_all_ports;
        self
    }
    pub fn privileged(&mut self, privileged: bool) -> &mut Self {
        self.privileged = privileged;
        self
    }
    pub fn readonly_rootfs(&mut self, readonly_rootfs: bool) -> &mut Self {
        self.readonly_rootfs = readonly_rootfs;
        self
    }
    pub fn dns(&mut self, dns: String) -> &mut Self {
        self.dns.push(dns);
        self
    }
    pub fn dns_options(&mut self, dns_option: String) -> &mut Self {
        self.dns_options.push(dns_option);
        self
    }
    pub fn dns_search(&mut self, dns_search: String) -> &mut Self {
        self.dns_search.push(dns_search);
        self
    }
    pub fn auto_remove(&mut self, auto_remove: bool) -> &mut Self {
        self.auto_remove = auto_remove;
        self
    }
    pub fn volumes_from(&mut self, volumes_from: String) -> &mut Self {
        self.volumes_from.push(volumes_from);
        self
    }
    pub fn cap_add(&mut self, cap_add: String) -> &mut Self {
        self.cap_add.push(cap_add);
        self
    }
    pub fn cap_drop(&mut self, cap_drop: String) -> &mut Self {
        self.cap_drop.push(cap_drop);
        self
    }
    pub fn group_add(&mut self, group_add: String) -> &mut Self {
        self.group_add.push(group_add);
        self
    }
    pub fn restart_policy(&mut self, restart_policy: RestartPolicy) -> &mut Self {
        self.restart_policy = restart_policy;
        self
    }
    pub fn network_mode(&mut self, network_mode: String) -> &mut Self {
        self.network_mode = network_mode;
        self
    }
    pub fn devices(&mut self, device: DeviceMapping) -> &mut Self {
        self.devices.push(device);
        self
    }
    pub fn sysctls(&mut self, key: &str, value: &str) -> &mut Self {
        self.sysctls.insert(key.to_owned(), value.to_owned());
        self
    }
    pub fn cgroup_parent(&mut self, cgroup_parent: String) -> &mut Self {
        self.cgroup_parent = cgroup_parent;
        self
    }
    pub fn volume_driver(&mut self, volume_driver: String) -> &mut Self {
        self.volume_driver = volume_driver;
        self
    }
    pub fn shm_size(&mut self, shm_size: u64) -> &mut Self {
        self.shm_size = shm_size;
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkingConfig {
    // TODO
}

/// request body of /containers/create api
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerCreateOptions {
    hostname: String,
    domainname: String,
    user: String,
    attach_stdin: bool,
    attach_stdout: bool,
    attach_stderr: bool,
    // exposed_ports: HashMap<String, Any>, not sure the type that this would need to be
    tty: bool,
    open_stdin: bool,
    stdin_once: bool,
    env: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cmd: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    entrypoint: Vec<String>,
    image: String,
    labels: HashMap<String, String>,
    // volumes: HashMap<String, Any>, not sure the type that this would need to be.
    // healthcheck: Not sure the type that this would be
    working_dir: PathBuf,
    network_disabled: bool,
    mac_address: String,
    on_build: Vec<String>,
    stop_signal: String,
    #[serde(with = "format::duration::DurationDelegate")]
    stop_timeout: Duration,
    host_config: Option<ContainerHostConfig>,
    networking_config: Option<NetworkingConfig>,
}

impl ContainerCreateOptions {
    pub fn new(image: &str) -> Self {
        Self {
            hostname: "".to_owned(),
            domainname: "".to_owned(),
            user: "".to_owned(),
            attach_stdin: false,
            attach_stdout: true,
            attach_stderr: true,
            tty: false,
            open_stdin: false,
            stdin_once: false,
            env: vec![],
            cmd: vec![],
            image: image.to_owned(),
            working_dir: PathBuf::new(),
            entrypoint: vec![],
            network_disabled: false,
            mac_address: "".to_owned(),
            on_build: vec![],
            labels: HashMap::new(),
            stop_signal: "SIGTERM".to_owned(),
            stop_timeout: Duration::from_secs(10),
            host_config: None,
            networking_config: None,
        }
    }

    pub fn hostname(&mut self, hostname: String) -> &mut Self {
        self.hostname = hostname;
        self
    }

    pub fn domainname(&mut self, domainname: String) -> &mut Self {
        self.domainname = domainname;
        self
    }

    pub fn user(&mut self, user: String) -> &mut Self {
        self.user = user;
        self
    }

    pub fn attach_stdin(&mut self, attach_stdin: bool) -> &mut Self {
        self.attach_stdin = attach_stdin;
        self
    }

    pub fn attach_stdout(&mut self, attach_stdout: bool) -> &mut Self {
        self.attach_stdout = attach_stdout;
        self
    }

    pub fn attach_stderr(&mut self, attach_stderr: bool) -> &mut Self {
        self.attach_stderr = attach_stderr;
        self
    }

    pub fn tty(&mut self, tty: bool) -> &mut Self {
        self.tty = tty;
        self
    }

    pub fn open_stdin(&mut self, open_stdin: bool) -> &mut Self {
        self.open_stdin = open_stdin;
        self
    }

    pub fn stdin_once(&mut self, stdin_once: bool) -> &mut Self {
        self.stdin_once = stdin_once;
        self
    }

    /// push back an envvar entry
    pub fn env(&mut self, env: String) -> &mut Self {
        self.env.push(env);
        self
    }

    /// push back a cmd argment
    pub fn cmd(&mut self, cmd: String) -> &mut Self {
        self.cmd.push(cmd);
        self
    }

    /// update entrypoint
    pub fn entrypoint(&mut self, entrypoint: Vec<String>) -> &mut Self {
        self.entrypoint = entrypoint;
        self
    }

    pub fn image(&mut self, image: String) -> &mut Self {
        self.image = image;
        self
    }

    /// add a label/value pair
    pub fn label(&mut self, key: String, value: String) -> &mut Self {
        self.labels.insert(key, value);
        self
    }

    pub fn working_dir(&mut self, working_dir: PathBuf) -> &mut Self {
        self.working_dir = working_dir;
        self
    }

    pub fn network_disabled(&mut self, network_disabled: bool) -> &mut Self {
        self.network_disabled = network_disabled;
        self
    }

    pub fn mac_address(&mut self, mac_address: String) -> &mut Self {
        self.mac_address = mac_address;
        self
    }

    pub fn on_build(&mut self, on_build: Vec<String>) -> &mut Self {
        self.on_build = on_build;
        self
    }

    pub fn stop_signal(&mut self, stop_signal: String) -> &mut Self {
        self.stop_signal = stop_signal;
        self
    }

    pub fn stop_timeout(&mut self, stop_timeout: Duration) -> &mut Self {
        self.stop_timeout = stop_timeout;
        self
    }

    pub fn host_config(&mut self, host_config: ContainerHostConfig) -> &mut Self {
        self.host_config = Some(host_config);
        self
    }

    pub fn networking_config(&mut self, networking_config: NetworkingConfig) -> &mut Self {
        self.networking_config = Some(networking_config);
        self
    }
}

mod format {
    pub mod duration {
        use std::time::Duration;

        #[derive(Serialize, Deserialize)]
        #[serde(remote = "Duration")]
        pub struct DurationDelegate(#[serde(getter = "Duration::as_secs")] u64);

        // Provide a conversion to construct the remote type.
        impl From<DurationDelegate> for Duration {
            fn from(def: DurationDelegate) -> Duration {
                Duration::new(def.0, 0)
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateContainerResponse {
    pub id: String,
    pub warnings: Option<Vec<String>>,
}

/// Response of the removing image api
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum RemovedImage {
    Untagged(String),
    Deleted(String),
}

/// Response of the prune image api
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct PrunedImages {
    #[serde(deserialize_with = "null_to_default")]
    ImagesDeleted: Vec<RemovedImage>,
    SpaceReclaimed: i64,
}
