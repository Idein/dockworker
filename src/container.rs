use crate::network::EndpointConfig;
use serde::de::{self, DeserializeOwned, Deserializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct Container {
    pub Id: String,
    pub Image: String,
    pub ImageID: String,
    pub State: String,
    pub Status: String,
    pub Command: String,
    pub Created: u64,
    pub Names: Vec<String>,
    pub Ports: Vec<Port>,
    pub SizeRw: Option<u64>, // I guess it is optional on Mac.
    pub SizeRootFs: Option<u64>,
    pub Labels: Option<HashMap<String, String>>,
    pub HostConfig: HostConfig,
    pub NetworkSettings: Option<SummaryNetworkSettings>,
    pub Mounts: Option<Vec<Mount>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_snake_case)]
pub struct Port {
    pub IP: Option<String>,
    pub PrivatePort: u64,
    pub PublicPort: Option<u64>,
    pub Type: PortType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PortType {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_snake_case)]
pub struct HostConfig {
    pub NetworkMode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct SummaryNetworkSettings {
    pub Networks: Option<HashMap<String, Option<Network>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ContainerInfo {
    pub AppArmorProfile: String,
    pub Args: Vec<String>,
    pub Config: Config,
    pub Created: String,
    pub Driver: String,
    // ExecIDs
    // GraphDriver
    // HostConfig
    pub HostnamePath: String,
    pub HostsPath: String,
    pub Id: String,
    pub Image: String,
    pub LogPath: String,
    pub MountLabel: String,
    pub Mounts: Vec<Mount>,
    pub Name: String,
    pub NetworkSettings: NetworkSettings,
    pub Path: String,
    pub ProcessLabel: String,
    pub ResolvConfPath: String,
    pub RestartCount: u64,
    pub State: State,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(non_snake_case)]
pub struct ExecProcessConfig {
    pub arguments: Vec<String>,
    pub entrypoint: String,
    pub privileged: bool,
    pub tty: bool,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(non_snake_case)]
pub struct ExecInfo {
    pub CanRemove: bool,
    pub ContainerID: String,
    pub DetachKeys: String,
    pub ExitCode: Option<u32>,
    pub ID: String,
    pub OpenStderr: bool,
    pub OpenStdin: bool,
    pub OpenStdout: bool,
    pub ProcessConfig: ExecProcessConfig,
    pub Running: bool,
    pub Pid: u64,
}

/// This type represents a `struct{}` in the Go code.
pub type UnspecifiedObject = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Config {
    pub AttachStderr: bool,
    pub AttachStdin: bool,
    pub AttachStdout: bool,
    #[serde(deserialize_with = "null_to_default")]
    pub Cmd: Vec<String>,
    pub Domainname: String,
    #[serde(deserialize_with = "null_to_default")]
    pub Entrypoint: Vec<String>,
    #[serde(deserialize_with = "null_to_default")]
    pub Env: Vec<String>,
    #[serde(default = "Default::default")]
    pub ExposedPorts: HashMap<String, UnspecifiedObject>,
    pub Hostname: String,
    pub Image: String,
    #[serde(deserialize_with = "null_to_default")]
    pub Labels: HashMap<String, String>,
    #[serde(deserialize_with = "null_to_default")]
    pub OnBuild: Vec<String>,
    pub OpenStdin: bool,
    pub StdinOnce: bool,
    pub Tty: bool,
    pub User: String,
    #[serde(deserialize_with = "null_to_default")]
    pub Volumes: HashMap<String, UnspecifiedObject>,
    pub WorkingDir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_snake_case)]
pub struct Mount {
    // Name (optional)
    // Driver (optional)
    pub Source: String,
    pub Destination: String,
    pub Mode: String,
    pub RW: bool,
    pub Propagation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct NetworkSettings {
    pub Bridge: String,
    pub EndpointID: String,
    pub Gateway: String,
    pub GlobalIPv6Address: String,
    pub GlobalIPv6PrefixLen: u32,
    pub HairpinMode: bool,
    pub IPAddress: String,
    pub IPPrefixLen: u32,
    pub IPv6Gateway: String,
    pub LinkLocalIPv6Address: String,
    pub LinkLocalIPv6PrefixLen: u32,
    pub MacAddress: String,
    /// network name to Network mapping
    pub Networks: HashMap<String, Network>,
    pub Ports: HashMap<String, Option<Vec<PortMapping>>>,
    pub SandboxID: String,
    pub SandboxKey: String,
    // These two are null in the current output.
    //pub SecondaryIPAddresses: ,
    //pub SecondaryIPv6Addresses: ,
}

pub type Network = EndpointConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_snake_case)]
pub struct PortMapping {
    pub HostIp: String,
    pub HostPort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LogMessage {
    pub Start: String,
    pub End: String,
    pub ExitCode: u64,
    pub Output: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, PartialOrd, Eq, Ord)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    /// Indicates there is no healthcheck
    NoHealthcheck,
    /// Indicates that the container is not yet ready
    Starting,
    /// Indicates that the container is running correctly
    Healthy,
    /// Indicates that the container has a problem
    Unhealthy,
}

impl fmt::Display for HealthState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HealthState::NoHealthcheck => write!(f, "none"),
            HealthState::Starting => write!(f, "starting"),
            HealthState::Healthy => write!(f, "healthy"),
            HealthState::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

impl<'de> Deserialize<'de> for HealthState {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

impl FromStr for HealthState {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "none" => Ok(HealthState::NoHealthcheck),
            "starting" => Ok(HealthState::Starting),
            "healthy" => Ok(HealthState::Healthy),
            "unhealthy" => Ok(HealthState::Unhealthy),
            _ => Err(format!("Cannot parse {s} into known HealthState variant!")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Health {
    pub Status: HealthState,
    pub FailingStreak: u64,
    pub Log: Vec<LogMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct State {
    pub Status: String,
    pub Running: bool,
    pub Paused: bool,
    pub Restarting: bool,
    pub OOMKilled: bool,
    pub Dead: bool,
    // I don't know whether PIDs can be negative here.  They're normally
    // positive, but sometimes negative PIDs are used in certain APIs.
    pub Pid: i64,
    pub ExitCode: i64,
    pub Error: String,
    pub StartedAt: String,
    pub FinishedAt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Health: Option<Health>,
}

impl std::fmt::Display for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.Id)
    }
}

impl std::fmt::Display for ContainerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.Id)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerStatus {
    Created,
    Restarting,
    Running,
    Removing,
    Paused,
    Exited,
    Dead,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Default)]
pub struct ContainerFilters {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    id: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    name: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    status: Vec<ContainerStatus>,
}

impl ContainerFilters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(&mut self, id: &str) -> &mut Self {
        self.id.push(id.to_owned());
        self
    }

    pub fn name(&mut self, name: &str) -> &mut Self {
        self.name.push(name.to_owned());
        self
    }

    pub fn status(&mut self, status: ContainerStatus) -> &mut Self {
        self.status.push(status);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContainerStdioType {
    Stdin,
    Stdout,
    Stderr,
}
/// response fragment of the attach container api
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct AttachResponseFrame {
    pub type_: ContainerStdioType,
    pub frame: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ExitStatus {
    StatusCode: i32,
}

impl ExitStatus {
    pub fn new(status_code: i32) -> Self {
        Self {
            StatusCode: status_code,
        }
    }

    pub fn into_inner(self) -> i32 {
        self.StatusCode
    }
}

impl From<i32> for ExitStatus {
    fn from(status_code: i32) -> Self {
        Self::new(status_code)
    }
}

fn null_to_default<'de, D, T>(de: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    let actual: Option<T> = Option::deserialize(de)?;
    Ok(actual.unwrap_or_default())
}

#[cfg(test)]
mod test {
    use super::*;

    // https://github.com/idein/dockworker/issues/84
    #[test]
    fn serde_network() {
        let network_settings_str = r#"{
            "Bridge": "",
            "SandboxID": "7c5ebca03e210aa5cdfa81206950a72584930291812fc82502ae0406efca60cf",
            "HairpinMode": false,
            "LinkLocalIPv6Address": "",
            "LinkLocalIPv6PrefixLen": 0,
            "Ports": {
                "3306/tcp": null
            },
            "SandboxKey": "/var/run/docker/netns/7c5ebcaace21",
            "SecondaryIPAddresses": null,
            "SecondaryIPv6Addresses": null,
            "EndpointID": "0a9c1de4bebcbf778248009fe2b4a747478e2136645563de7ba8d48f287d9388",
            "Gateway": "172.11.0.1",
            "GlobalIPv6Address": "",
            "GlobalIPv6PrefixLen": 0,
            "IPAddress": "171.11.0.70",
            "IPPrefixLen": 16,
            "IPv6Gateway": "",
            "MacAddress": "01:42:0c:11:c0:f9",
            "Networks": {
                "bridge": {
                    "IPAMConfig": {},
                    "Links": null,
                    "Aliases": null,
                    "NetworkID": "c6bcc45303b33fb881911c25e755da483291123b0a8099e42b2226bcd4f2d549",
                    "EndpointID": "0a9c1de4bebcbf778248009fe2b4a74432012136645563de7ba8719e987d9388",
                    "Gateway": "172.11.0.1",
                    "IPAddress": "172.11.0.70",
                    "IPPrefixLen": 16,
                    "IPv6Gateway": "",
                    "GlobalIPv6Address": "",
                    "GlobalIPv6PrefixLen": 0,
                    "MacAddress": "01:42:0c:11:c0:f9",
                    "DriverOpts": null
                }
            }
        }"#;
        let network_settings: NetworkSettings = serde_json::from_str(network_settings_str).unwrap();
        let network_settings_json: serde_json::Value =
            serde_json::to_value(network_settings).unwrap();

        let network_settings_serde: serde_json::Value = {
            let network_settings_str = serde_json::to_string(&network_settings_json).unwrap();
            serde_json::from_str(&network_settings_str).unwrap()
        };

        assert_eq!(network_settings_json, network_settings_serde);
    }
}
