//! Options which can be passed to various `Docker` commands.
#![allow(clippy::new_without_default)]

use crate::network;
use serde::de::{DeserializeOwned, Deserializer};
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use url::{self, form_urlencoded};

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

/// Restart policy of a container.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct RestartPolicy {
    /// Restart type
    /// This option can be "no", "always", "on-failure" or "unless-stopped"
    pub Name: String,
    /// Maximum retry count. This value is used only when "on-failure" mode
    pub MaximumRetryCount: u16,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::new("no".to_owned(), 0)
    }
}

impl RestartPolicy {
    pub fn new(name: String, maximum_retry_count: u16) -> Self {
        RestartPolicy {
            Name: name,
            MaximumRetryCount: maximum_retry_count,
        }
    }

    pub fn no() -> Self {
        Self::new("no".to_owned(), 0)
    }

    pub fn always() -> Self {
        Self::new("always".to_owned(), 0)
    }

    pub fn on_failure() -> Self {
        Self::new("on-failure".to_owned(), 10)
    }

    pub fn unless_stopped() -> Self {
        Self::new("unless-stopped".to_owned(), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_logconfig() {
        let cfg = LogConfig::new(LogConfigType::JsonFile);
        let json = serde_json::to_string(&cfg).unwrap();
        let json_cfg = serde_json::from_str(&json).unwrap();
        assert_eq!(&cfg, &json_cfg);
    }

    #[test]
    fn serde_logconfig_with_opt() {
        let config = {
            let mut cfg = HashMap::new();
            cfg.insert("tagA".to_string(), "valueA".to_string());
            cfg.insert("tagB".to_string(), "valueB".to_string());
            cfg
        };
        let cfg = LogConfig {
            r#type: LogConfigType::JsonFile,
            config,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let json_cfg = serde_json::from_str(&json).unwrap();
        assert_eq!(&cfg, &json_cfg);
    }

    #[test]
    fn deser_restart_policy() {
        let no = r#"{"MaximumRetryCount":0, "Name":"no"}"#;
        assert_eq!(RestartPolicy::default(), serde_json::from_str(no).unwrap());
        assert_eq!(RestartPolicy::no(), serde_json::from_str(no).unwrap());
        let always = r#"{"MaximumRetryCount":0, "Name":"always"}"#;
        assert_eq!(
            RestartPolicy::always(),
            serde_json::from_str(always).unwrap()
        );
        let onfailure = r#"{"MaximumRetryCount":10, "Name":"on-failure"}"#;
        assert_eq!(
            RestartPolicy::on_failure(),
            serde_json::from_str(onfailure).unwrap()
        );
        let unlessstopped = r#"{"MaximumRetryCount":0, "Name":"unless-stopped"}"#;
        assert_eq!(
            RestartPolicy::unless_stopped(),
            serde_json::from_str(unlessstopped).unwrap()
        );
    }

    #[test]
    fn iso_restart_policy() {
        let no = RestartPolicy::default();
        assert_eq!(
            serde_json::from_str::<RestartPolicy>(&serde_json::to_string(&no).unwrap()).unwrap(),
            no
        );
        let always = RestartPolicy::new("always".to_owned(), 0);
        assert_eq!(
            serde_json::from_str::<RestartPolicy>(&serde_json::to_string(&always).unwrap())
                .unwrap(),
            always
        );
        let onfailure = RestartPolicy::new("on-failure".to_owned(), 10);
        assert_eq!(
            serde_json::from_str::<RestartPolicy>(&serde_json::to_string(&onfailure).unwrap())
                .unwrap(),
            onfailure
        );
        let unlessstopped = RestartPolicy::new("unless-stopped".to_owned(), 0);
        assert_eq!(
            serde_json::from_str::<RestartPolicy>(&serde_json::to_string(&unlessstopped).unwrap())
                .unwrap(),
            unlessstopped
        );
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
    binds: Option<Vec<String>>,
    tmpfs: Option<HashMap<String, String>>,
    links: Option<Vec<String>>,
    memory: Option<u64>,
    memory_swap: Option<u64>,
    memory_reservation: Option<u64>,
    kernel_memory: Option<u64>,
    cpu_percent: Option<u64>,
    cpu_shares: Option<u64>,
    cpu_period: Option<u64>,
    cpu_quota: Option<u64>,
    cpuset_cpus: Option<String>,
    io_maximum_bandwidth: Option<u64>,
    io_maximum_ops: Option<u64>,
    blkio_weight: Option<u64>,
    memory_swappiness: Option<i32>,
    oom_kill_disable: Option<bool>,
    oom_score_adj: Option<u16>,
    pid_mode: Option<String>,
    pids_limit: Option<i16>,
    port_bindings: Option<PortBindings>,
    publish_all_ports: Option<bool>,
    privileged: Option<bool>,
    readonly_rootfs: Option<bool>,
    dns: Option<Vec<String>>,
    dns_options: Option<Vec<String>>,
    dns_search: Option<Vec<String>>,
    auto_remove: Option<bool>,
    volumes_from: Option<Vec<String>>,
    cap_add: Option<Vec<String>>,
    cap_drop: Option<Vec<String>>,
    group_add: Option<Vec<String>>,
    restart_policy: Option<RestartPolicy>,
    network_mode: Option<String>,
    devices: Option<Vec<DeviceMapping>>,
    sysctls: Option<HashMap<String, String>>,
    runtime: Option<String>,
    log_config: Option<LogConfig>,
    cgroup_parent: Option<String>,
    volume_driver: Option<String>,
    shm_size: Option<u64>,
    userns_mode: Option<String>,
}

impl ContainerHostConfig {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn userns_mode(&mut self, userns_mode: String) -> &mut Self {
        self.userns_mode = Some(userns_mode);
        self
    }

    pub fn binds(&mut self, binds: Vec<String>) -> &mut Self {
        self.binds = Some(binds);
        self
    }

    pub fn tmpfs(&mut self, tmpfs: HashMap<String, String>) -> &mut Self {
        self.tmpfs = Some(tmpfs);
        self
    }

    pub fn links(&mut self, links: Vec<String>) -> &mut Self {
        self.links = Some(links);
        self
    }

    pub fn memory(&mut self, memory: u64) -> &mut Self {
        self.memory = Some(memory);
        self
    }

    pub fn memory_swap(&mut self, memory_swap: u64) -> &mut Self {
        self.memory_swap = Some(memory_swap);
        self
    }

    pub fn memory_reservation(&mut self, memory_reservation: u64) -> &mut Self {
        self.memory_reservation = Some(memory_reservation);
        self
    }

    pub fn kernel_memory(&mut self, kernel_memory: u64) -> &mut Self {
        self.kernel_memory = Some(kernel_memory);
        self
    }

    pub fn cpu_percent(&mut self, cpu_percent: u64) -> &mut Self {
        self.cpu_percent = Some(cpu_percent);
        self
    }

    pub fn cpu_shares(&mut self, cpu_shares: u64) -> &mut Self {
        self.cpu_shares = Some(cpu_shares);
        self
    }

    pub fn cpu_period(&mut self, cpu_period: u64) -> &mut Self {
        self.cpu_period = Some(cpu_period);
        self
    }

    pub fn cpu_quota(&mut self, cpu_quota: u64) -> &mut Self {
        self.cpu_quota = Some(cpu_quota);
        self
    }

    pub fn cpuset_cpus(&mut self, cpuset_cpus: String) -> &mut Self {
        self.cpuset_cpus = Some(cpuset_cpus);
        self
    }

    pub fn io_maximum_bandwidth(&mut self, io_maximum_bandwidth: u64) -> &mut Self {
        self.io_maximum_bandwidth = Some(io_maximum_bandwidth);
        self
    }

    pub fn io_maximum_ops(&mut self, io_maximum_ops: u64) -> &mut Self {
        self.io_maximum_ops = Some(io_maximum_ops);
        self
    }

    pub fn blkio_weight(&mut self, blkio_weight: u64) -> &mut Self {
        self.blkio_weight = Some(blkio_weight);
        self
    }

    pub fn memory_swappiness(&mut self, memory_swappiness: i32) -> &mut Self {
        self.memory_swappiness = Some(memory_swappiness);
        self
    }

    pub fn oom_kill_disable(&mut self, oom_kill_disable: bool) -> &mut Self {
        self.oom_kill_disable = Some(oom_kill_disable);
        self
    }

    pub fn oom_score_adj(&mut self, oom_score_adj: u16) -> &mut Self {
        self.oom_score_adj = Some(oom_score_adj);
        self
    }

    pub fn pid_mode(&mut self, pid_mode: String) -> &mut Self {
        self.pid_mode = Some(pid_mode);
        self
    }

    pub fn pids_limit(&mut self, pids_limit: i16) -> &mut Self {
        self.pids_limit = Some(pids_limit);
        self
    }

    pub fn publish_all_ports(&mut self, publish_all_ports: bool) -> &mut Self {
        self.publish_all_ports = Some(publish_all_ports);
        self
    }

    pub fn privileged(&mut self, privileged: bool) -> &mut Self {
        self.privileged = Some(privileged);
        self
    }

    pub fn readonly_rootfs(&mut self, readonly_rootfs: bool) -> &mut Self {
        self.readonly_rootfs = Some(readonly_rootfs);
        self
    }

    pub fn dns(&mut self, dns: Vec<String>) -> &mut Self {
        self.dns = Some(dns);
        self
    }

    pub fn dns_options(&mut self, dns_options: Vec<String>) -> &mut Self {
        self.dns_options = Some(dns_options);
        self
    }

    pub fn dns_search(&mut self, dns_search: Vec<String>) -> &mut Self {
        self.dns_search = Some(dns_search);
        self
    }

    pub fn auto_remove(&mut self, auto_remove: bool) -> &mut Self {
        self.auto_remove = Some(auto_remove);
        self
    }

    pub fn volumes_from(&mut self, volumes_from: Vec<String>) -> &mut Self {
        self.volumes_from = Some(volumes_from);
        self
    }

    pub fn cap_add(&mut self, cap_add: Vec<String>) -> &mut Self {
        self.cap_add = Some(cap_add);
        self
    }

    pub fn cap_drop(&mut self, cap_drop: Vec<String>) -> &mut Self {
        self.cap_drop = Some(cap_drop);
        self
    }

    pub fn group_add(&mut self, group_add: Vec<String>) -> &mut Self {
        self.group_add = Some(group_add);
        self
    }

    pub fn restart_policy(&mut self, restart_policy: RestartPolicy) -> &mut Self {
        self.restart_policy = Some(restart_policy);
        self
    }

    pub fn network_mode(&mut self, network_mode: String) -> &mut Self {
        self.network_mode = Some(network_mode);
        self
    }

    pub fn devices(&mut self, devices: Vec<DeviceMapping>) -> &mut Self {
        self.devices = Some(devices);
        self
    }

    pub fn sysctls(&mut self, sysctls: HashMap<String, String>) -> &mut Self {
        self.sysctls = Some(sysctls);
        self
    }

    pub fn runtime(&mut self, runtime: String) -> &mut Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn log_config(&mut self, log_config: LogConfig) -> &mut Self {
        self.log_config = Some(log_config);
        self
    }

    pub fn cgroup_parent(&mut self, cgroup_parent: String) -> &mut Self {
        self.cgroup_parent = Some(cgroup_parent);
        self
    }

    pub fn volume_driver(&mut self, volume_driver: String) -> &mut Self {
        self.volume_driver = Some(volume_driver);
        self
    }

    pub fn shm_size(&mut self, shm_size: u64) -> &mut Self {
        self.shm_size = Some(shm_size);
        self
    }

    pub fn port_bindings(&mut self, port_bindings: PortBindings) -> &mut Self {
        self.port_bindings = Some(port_bindings);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[derive(Default)]
pub struct LogConfig {
    pub r#type: LogConfigType,
    pub config: HashMap<String, String>,
}

impl LogConfig {
    pub fn new(r#type: LogConfigType) -> Self {
        Self {
            r#type,
            config: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum LogConfigType {
    JsonFile,
    Syslog,
    #[default]
    Journald,
    Gelf,
    Fluentd,
    Awslogs,
    Splunk,
    Etwlogs,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkingConfig {
    pub endpoints_config: EndpointsConfig,
}

/// network name to EndpointConfig
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointsConfig(HashMap<String, network::EndpointConfig>);

impl From<HashMap<String, network::EndpointConfig>> for EndpointsConfig {
    fn from(endpoints: HashMap<String, network::EndpointConfig>) -> Self {
        EndpointsConfig(endpoints)
    }
}

#[derive(Debug, Clone)]
pub struct ContainerLogOptions {
    pub stdout: bool,
    pub stderr: bool,
    pub since: Option<i64>,
    pub timestamps: Option<bool>,
    pub tail: Option<i64>,
    pub follow: bool,
}

impl ContainerLogOptions {
    pub(crate) fn to_url_params(&self) -> String {
        let mut param = url::form_urlencoded::Serializer::new(String::new());
        param.append_pair("stdout", &self.stdout.to_string());
        param.append_pair("stderr", &self.stderr.to_string());
        param.append_pair("follow", &self.follow.to_string());
        if let Some(since) = self.since {
            param.append_pair("since", &since.to_string());
        }
        if let Some(timestamps) = self.timestamps {
            param.append_pair("timestamps", &timestamps.to_string());
        }
        if let Some(tail) = self.tail {
            param.append_pair("tail", &tail.to_string());
        }
        param.finish()
    }
}

impl Default for ContainerLogOptions {
    fn default() -> Self {
        ContainerLogOptions {
            stdout: true,
            stderr: true,
            follow: false,
            since: None,
            timestamps: None,
            tail: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContainerBuildOptions {
    /// Path within the build context to the Dockerfile.
    /// This is ignored if remote is specified and points to an external Dockerfile.
    pub dockerfile: String,

    /// A name and optional tag to apply to the image in the name:tag format.
    /// If you omit the tag the default latest value is assumed. You can provide several t parameters.
    pub t: Vec<String>,

    /// Extra hosts to add to /etc/hosts
    pub extrahosts: Option<String>,

    /// A Git repository URI or HTTP/HTTPS context URI
    pub remote: Option<String>,

    /// Suppress verbose build output.
    pub q: bool,

    /// Do not use the cache when building the image.
    pub nocache: bool,

    /// JSON array of images used for build cache resolution.
    pub cachefrom: Option<Vec<String>>,

    /// Attempt to pull the image even if an older image exists locally.
    pub pull: Option<String>,

    /// Remove intermediate containers after a successful build.
    pub rm: bool,

    /// Always remove intermediate containers, even upon failure.
    pub forcerm: bool,

    /// Set memory limit for build.
    pub memory: Option<u64>,

    /// Total memory (memory + swap). Set as -1 to disable swap.
    pub memswap: Option<i64>,

    /// CPU shares (relative weight).
    pub cpushares: Option<u64>,

    /// CPUs in which to allow execution (e.g., 0-3, 0,1).
    pub cpusetcpus: Option<String>,

    /// The length of a CPU period in microseconds.
    pub cpuperiod: Option<u64>,

    /// Microseconds of CPU time that the container can get in a CPU period.
    pub cpuquota: Option<u64>,

    /// JSON map of string pairs for build-time variables.
    /// This is not meant for passing secret values.
    pub buildargs: Option<HashMap<String, String>>,

    /// Size of /dev/shm in bytes. The size must be greater than 0. If omitted the system uses 64MB.
    pub shmsize: Option<u64>,

    /// Squash the resulting images layers into a single layer. (Experimental release only.)
    pub squash: Option<bool>,

    /// Arbitrary key/value labels to set on the image, as a JSON map of string pairs.
    pub labels: Option<HashMap<String, String>>,

    /// Sets the networking mode for the run commands during build.
    /// Supported standard values are: bridge, host, none, and container:<name|id>.
    /// Any other value is taken as a custom network's name to which this container should connect to.
    pub networkmode: Option<String>,

    /// Platform in the format os[/arch[/variant]]
    pub platform: String,
}

impl ContainerBuildOptions {
    /// Convert to URL parameters.
    pub fn to_url_params(&self) -> String {
        let mut params = form_urlencoded::Serializer::new(String::new());
        params.append_pair("dockerfile", &self.dockerfile);
        for tag in &self.t {
            params.append_pair("t", tag);
        }
        if let Some(ref extrahosts) = self.extrahosts {
            params.append_pair("extrahosts", extrahosts);
        }
        if let Some(ref remote) = self.remote {
            params.append_pair("remote", remote);
        }
        if self.q {
            params.append_pair("q", "true");
        }
        if self.nocache {
            params.append_pair("nocache", "true");
        }
        if let Some(ref cachefrom) = self.cachefrom {
            params.append_pair("cachefrom", &serde_json::to_string(&cachefrom).unwrap());
        }
        if let Some(ref pull) = self.pull {
            params.append_pair("pull", pull);
        }
        if self.rm {
            params.append_pair("rm", "true");
        }
        if self.forcerm {
            params.append_pair("forcerm", "true");
        }
        if let Some(ref memory) = self.memory {
            params.append_pair("memory", &memory.to_string());
        }
        if let Some(ref memswap) = self.memswap {
            params.append_pair("memswap", &memswap.to_string());
        }
        if let Some(ref cpushares) = self.cpushares {
            params.append_pair("cpushares", &cpushares.to_string());
        }
        if let Some(ref cpusetcpus) = self.cpusetcpus {
            params.append_pair("cpusetcpus", cpusetcpus);
        }
        if let Some(ref cpuperiod) = self.cpuperiod {
            params.append_pair("cpuperiod", &cpuperiod.to_string());
        }
        if let Some(ref cpuquota) = self.cpuquota {
            params.append_pair("cpuquota", &cpuquota.to_string());
        }
        if let Some(ref buildargs) = self.buildargs {
            params.append_pair(
                "buildargs",
                &serde_json::to_string(&buildargs).expect("Json parsing of buildargs param"),
            );
        }
        if let Some(ref shmsize) = self.shmsize {
            params.append_pair("shmsize", &shmsize.to_string());
        }
        if let Some(ref squash) = self.squash {
            params.append_pair("squash", &squash.to_string());
        }
        if let Some(ref labels) = self.labels {
            params.append_pair(
                "labels",
                &serde_json::to_string(&labels).expect("Json parsing of labels param"),
            );
        }
        if let Some(ref networkmode) = self.networkmode {
            params.append_pair("networkmode", networkmode);
        }
        params.append_pair("platform", &self.platform);
        params.finish()
    }
}

impl Default for ContainerBuildOptions {
    fn default() -> Self {
        ContainerBuildOptions {
            dockerfile: String::from("Dockerfile"),
            t: Vec::new(),
            extrahosts: None,
            remote: None,
            q: false,
            nocache: false,
            cachefrom: None,
            pull: None,
            rm: true,
            forcerm: false,
            memory: None,
            memswap: None,
            cpushares: None,
            cpusetcpus: None,
            cpuperiod: None,
            cpuquota: None,
            buildargs: None,
            shmsize: None,
            squash: Some(false),
            labels: None,
            networkmode: None,
            platform: String::new(),
        }
    }
}
#[derive(Debug, Clone, Default)]
pub struct ExposedPorts(pub Vec<(u16, String)>);

impl serde::Serialize for ExposedPorts {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = HashMap::new();
        for (port, protocol) in &self.0 {
            map.insert(
                format!("{}/{}", port, protocol).clone(),
                serde_json::Value::Object(serde_json::Map::new()),
            );
        }
        map.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for ExposedPorts {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = HashMap::<String, serde_json::Value>::deserialize(deserializer)?;
        let keys = map
            .keys()
            .map(|k| {
                let mut parts = k.split('/');
                let port = parts.next().unwrap().parse().unwrap();
                let protocol = parts.next().unwrap().to_owned();
                (port, protocol)
            })
            .collect();
        Ok(ExposedPorts(keys))
    }
}

#[test]
fn test_exposed_ports() {
    let ports = ExposedPorts(vec![
        (80, "tcp".to_owned()),
        (443, "tcp".to_owned()),
        (8080, "tcp".to_owned()),
        (8443, "tcp".to_owned()),
    ]);
    let json = serde_json::to_string(&ports).unwrap();
    // hashmapのkey順序は不定であるため,json_valueに変換してから比較が必要
    let result_json = serde_json::Value::from_str(&json).unwrap();
    let expected_json =
        serde_json::Value::from_str(r#"{"80/tcp":{},"443/tcp":{},"8080/tcp":{},"8443/tcp":{}}"#)
            .unwrap();

    assert_eq!(result_json, expected_json);

    let ports: ExposedPorts = serde_json::from_str(&json).unwrap();
    let result: HashSet<&(u16, String)> = HashSet::from_iter(ports.0.iter());
    // hashmapのkey順序は不定であるため,hash_setに変換してから比較する
    assert_eq!(
        result,
        HashSet::from_iter(
            vec![
                (80, "tcp".to_owned()),
                (443, "tcp".to_owned()),
                (8080, "tcp".to_owned()),
                (8443, "tcp".to_owned())
            ]
            .iter()
        )
    );
}

#[derive(Debug, Clone, Default)]
pub struct PortBindings(pub Vec<(u16, String, u16)>);

impl serde::Serialize for PortBindings {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = HashMap::new();
        for (container_port, protocol, host_port) in &self.0 {
            map.insert(
                format!("{}/{}", container_port, protocol).clone(),
                vec![serde_json::json!({"HostPort": host_port.to_string()})],
            );
        }
        map.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for PortBindings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = HashMap::<String, serde_json::Value>::deserialize(deserializer)?;
        let tuples = map
            .keys()
            .map(|k| {
                let mut parts = k.split('/');
                let port = parts.next().unwrap().parse().unwrap();
                let protocol = parts.next().unwrap().to_owned();
                let host_port = map
                    .get(k)
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .first()
                    .unwrap()
                    .get("HostPort")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .parse()
                    .unwrap();
                (port, protocol, host_port)
            })
            .collect();
        Ok(PortBindings(tuples))
    }
}

#[test]
fn test_port_bindings() {
    let ports = PortBindings(vec![
        (80, "tcp".to_owned(), 8080),
        (443, "tcp".to_owned(), 8000),
    ]);
    let json = serde_json::to_string(&ports).unwrap();
    // hashmapのkey順序は不定であるため,json_valueに変換してから比較が必要
    let result_json = serde_json::Value::from_str(&json).unwrap();
    let expected_json = serde_json::Value::from_str(
        r#"{"80/tcp":[{"HostPort":"8080"}],"443/tcp":[{"HostPort":"8000"}]}"#,
    )
    .unwrap();

    assert_eq!(result_json, expected_json);

    let ports: PortBindings = serde_json::from_str(&json).unwrap();
    let result: HashSet<&(u16, String, u16)> = HashSet::from_iter(ports.0.iter());
    // hashmapのkey順序は不定であるため,hash_setに変換してから比較する
    assert_eq!(
        result,
        HashSet::from_iter(
            vec![(80, "tcp".to_owned(), 8080), (443, "tcp".to_owned(), 8000),].iter()
        )
    );
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
    exposed_ports: Option<ExposedPorts>,
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
            exposed_ports: None,
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

    pub fn exposed_ports(&mut self, exposed_ports: ExposedPorts) -> &mut Self {
        self.exposed_ports = Some(exposed_ports);
        self
    }
}

mod format {
    pub mod duration {
        use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateExecResponse {
    pub id: String,
}

/// request body of /containers/Create an exec instance
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateExecOptions {
    attach_stdin: bool,
    attach_stdout: bool,
    attach_stderr: bool,
    detach_keys: String,
    tty: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    env: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cmd: Vec<String>,
    privileged: bool,
    user: String,
    working_dir: PathBuf,
}

impl CreateExecOptions {
    pub fn new() -> Self {
        Self {
            attach_stdin: false,
            attach_stdout: true,
            attach_stderr: true,
            detach_keys: "".to_owned(),
            tty: false,
            env: vec![],
            cmd: vec![],
            privileged: false,
            user: "".to_owned(),
            working_dir: PathBuf::new(),
        }
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

    pub fn env(&mut self, env: String) -> &mut Self {
        self.env.push(env);
        self
    }

    /// push back a cmd argment
    pub fn cmd(&mut self, cmd: String) -> &mut Self {
        self.cmd.push(cmd);
        self
    }

    pub fn privileged(&mut self, privileged: bool) -> &mut Self {
        self.privileged = privileged;
        self
    }

    pub fn user(&mut self, user: String) -> &mut Self {
        self.user = user;
        self
    }

    pub fn working_dir(&mut self, working_dir: PathBuf) -> &mut Self {
        self.working_dir = working_dir;
        self
    }
}

/// request body of /exec/start an exec instance
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct StartExecOptions {
    detach: bool,
    tty: bool,
}

impl StartExecOptions {
    pub fn new() -> Self {
        Self {
            detach: false,
            tty: false,
        }
    }

    pub fn detach(&mut self, detach: bool) -> &mut Self {
        self.detach = detach;
        self
    }

    pub fn tty(&mut self, tty: bool) -> &mut Self {
        self.tty = tty;
        self
    }
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

/// Response of the history image api
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ImageLayer {
    pub id: Option<String>,
    pub created: i64,
    pub created_by: String,
    pub tags: Option<Vec<String>>,
    pub size: u64,
    pub comment: String,
}

#[derive(Debug, PartialEq, PartialOrd, Serialize, Default)]
pub struct EventFilters {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    config: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    container: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    daemon: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    event: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    image: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    label: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    network: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    node: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    plugin: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    scope: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    secret: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    service: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "type")]
    type_: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    volume: Vec<String>,
}

impl EventFilters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn config(&mut self, config: &str) -> &mut Self {
        self.config.push(config.to_owned());
        self
    }

    pub fn container(&mut self, container: &str) -> &mut Self {
        self.container.push(container.to_owned());
        self
    }

    pub fn daemon(&mut self, daemon: &str) -> &mut Self {
        self.daemon.push(daemon.to_owned());
        self
    }

    pub fn event(&mut self, event: &str) -> &mut Self {
        self.event.push(event.to_owned());
        self
    }

    pub fn image(&mut self, image: &str) -> &mut Self {
        self.image.push(image.to_owned());
        self
    }

    pub fn label(&mut self, label: &str) -> &mut Self {
        self.label.push(label.to_owned());
        self
    }

    pub fn network(&mut self, network: &str) -> &mut Self {
        self.network.push(network.to_owned());
        self
    }

    pub fn node(&mut self, node: &str) -> &mut Self {
        self.node.push(node.to_owned());
        self
    }

    pub fn plugin(&mut self, plugin: &str) -> &mut Self {
        self.plugin.push(plugin.to_owned());
        self
    }

    pub fn scope(&mut self, scope: &str) -> &mut Self {
        self.scope.push(scope.to_owned());
        self
    }

    pub fn secret(&mut self, secret: &str) -> &mut Self {
        self.secret.push(secret.to_owned());
        self
    }

    pub fn service(&mut self, service: &str) -> &mut Self {
        self.service.push(service.to_owned());
        self
    }

    pub fn type_(&mut self, type_: &str) -> &mut Self {
        self.type_.push(type_.to_owned());
        self
    }

    pub fn volume(&mut self, volume: &str) -> &mut Self {
        self.volume.push(volume.to_owned());
        self
    }
}
