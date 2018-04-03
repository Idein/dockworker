//! Options which can be passed to various `Docker` commands.

use url::form_urlencoded;
use std::collections::HashMap;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RestartPolicy {
    name: String,
    maximum_retry_count: u16 // TODO: Maybe this can be smaller?
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    cpu_shares: u64, // TODO: I don't have a single clue what this even is
    cpu_period: u64, // TODO: Still no clue
    cpu_quota: u64, // TODO: Still no clue
    cpuset_cpus: String, // TODO: Still no clue
    io_maximum_bandwidth: u64, // TODO: Still no clue
    io_maximum_ops: u64, // TODO: Still no clue
    blkio_weight: u64, // TODO: Still no clue
    // blkio_weight_device: not sure the type of this. the provided is [{}]
    // blkio_weight_read_bps: not sure the type of this. the provided is [{}],
    // blkio_weight_read_iops: not sure the type of this. the provided is [{}]
    // blkio_weight_write_bps: not sure the type of this. the provided is [{}]
    // blkio_weight_write_iops: not sure the type of this. the provided is [{}]
    memory_swappiness: u16, // TODO: Maybe this can be smaller?
    oom_kill_disable: bool,
    oom_score_adj: u16, // TODO: Maybe this can be smaller?
    pid_mode: String,
    pids_limit: i16,
    port_bindings: HashMap<String, Vec<HashMap<String, String>>>,
    publish_all_ports: bool,
    privileged: bool,
    readonly_root_fs: bool,
    dns: Vec<String>,
    dns_options: Vec<String>,
    dns_search: Vec<String>,
    // extra_hosts: Not sure the type of this. the provided is null
    volumes_from: Vec<String>,
    cap_add: Vec<String>,
    cap_drop: Vec<String>,
    group_add: Vec<String>,
    restart_policy: RestartPolicy,
    network_mode: String,
    devices: Vec<String>,
    sysctls: HashMap<String, String>,
    // ulimits: TODO: Not sure the type of this. the provided is [{}]
    // log_config: TODO: not sure the type of this. the provided makes no sense
    // security_opt: TODO: Not sure the type of this. The provided is []
    // storage_opt: TODO: Not sure the type of this. the provided is {}
    cgroup_parent: String,
    volume_driver: String,
    shm_size: u64
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkingConfig {
    // TODO
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerCreateOptions {
    hostname: String,
    domain_name: String,
    user: String,
    attach_stdin: bool,
    attach_stdout: bool,
    attach_stderr: bool,
    tty: bool,
    open_stdin: bool,
    stdin_once: bool,
    env: Vec<String>,
    cmd: Vec<String>,
    entrypoint: String,
    image: String,
    labels: HashMap<String, String>,
    // volumes: HashMap<String, Any>, not sure the type that this would need to be.
    // healthcheck: Not sure the type that this would be
    working_dir: String,
    network_disable: bool,
    mac_address: String,
    // exposed_ports: HashMap<String, Any>, not sure the type that this would need to be
    stop_signal: String,
    host_config: ContainerHostConfig,
    networking_config: NetworkingConfig
}
