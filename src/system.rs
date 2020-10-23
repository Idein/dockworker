use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

struct NumToBoolVisitor;

impl<'de> Visitor<'de> for NumToBoolVisitor {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("0 or 1 or true or false")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value)
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value != 0)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value != 0)
    }
}

fn num_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(NumToBoolVisitor)
}

/// response of /info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SystemInfo {
    pub ID: String,
    pub Containers: u64,
    // pub ContainersRunning: u64,
    // pub ContainersPaused: u64,
    // pub ContainersStopped: u64,
    pub Images: u64,
    pub Driver: String,
    pub DriverStatus: Vec<(String, String)>,
    pub DockerRootDir: PathBuf,
    #[serde(deserialize_with = "num_to_bool")]
    pub MemoryLimit: bool,
    #[serde(deserialize_with = "num_to_bool")]
    pub SwapLimit: bool,
    // pub KernelMemory: bool,
    // pub OomKillDisable: bool,
    #[serde(deserialize_with = "num_to_bool")]
    pub IPv4Forwarding: bool,
    // pub BridgeNfIptables: bool,
    // pub BridgeNfIp6tables: bool,
    #[serde(deserialize_with = "num_to_bool")]
    pub Debug: bool,
    pub NFd: u64,
    pub NGoroutines: u64,
    // pub SystemTime: String,
    // pub LoggingDriver: String,
    // pub CgroupDriver: String,
    pub NEventsListener: u64,
    // pub KernelVersion: String,
    pub OperatingSystem: String,
    // pub OSType: String,
    // pub Architecture: String,
    pub NCPU: u64,
    pub MemTotal: u64,
    pub IndexServerAddress: String,
    // pub HttpProxy: String,
    // pub HttpsProxy: String,
    // pub NoProxy: String,
    // pub Name: String,
    pub Labels: Option<Vec<String>>,
    // pub ServerVersion: String,
}

/// Type of the response of `/auth` api
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct AuthToken {
    Status: String,
    IdentityToken: String,
}

impl AuthToken {
    #[allow(dead_code)]
    pub fn token(&self) -> String {
        self.IdentityToken.clone()
    }
}
