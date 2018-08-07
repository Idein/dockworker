//! Options which can be passed to various `Docker` commands.

use std::path::PathBuf;
use std::time::Duration;

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
    networking_config: Option<NetworkingConfig>
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
            networking_config: None
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

    pub fn attach_stdout(&mut self, attach_stdout: bool,) -> &mut Self {
        self.attach_stdout = attach_stdout;
        self
    }

    pub fn attach_stderr(&mut self, attach_stderr: bool,) -> &mut Self {
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
    use serde::de::Deserializer;
    use serde::ser::Serializer;

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

