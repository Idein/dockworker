use std::path::PathBuf;

/// response of /info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SystemInfo {
    pub ID: String,
    pub Containers: u64,
    pub ContainersRunning: u64,
    pub ContainersPaused: u64,
    pub ContainersStopped: u64,
    pub Images: u64,
    pub Driver: String,
    pub DriverStatus: Vec<(String, String)>,
    pub DockerRootDir: PathBuf,
    pub MemoryLimit: bool,
    pub SwapLimit: bool,
    pub KernelMemory: bool,
    pub OomKillDisable: bool,
    pub IPv4Forwarding: bool,
    pub BridgeNfIptables: bool,
    pub BridgeNfIp6tables: bool,
    pub Debug: bool,
    pub NFd: u64,
    pub NGoroutines: u64,
    pub SystemTime: String,
    pub LoggingDriver: String,
    pub CgroupDriver: String,
    pub NEventsListener: u64,
    pub KernelVersion: String,
    pub OperatingSystem: String,
    pub OSType: String,
    pub Architecture: String,
    pub NCPU: u64,
    pub MemTotal: u64,
    pub IndexServerAddress: String,
    pub HttpProxy: String,
    pub HttpsProxy: String,
    pub NoProxy: String,
    pub Name: String,
    pub Labels: Option<Vec<String>>,
    pub ServerVersion: String,
}

