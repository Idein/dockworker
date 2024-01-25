#![allow(clippy::new_without_default)]
use log::warn;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::net::Ipv4Addr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Network {
    pub Name: String,
    pub Id: String,
    pub Created: String,
    pub Scope: String,
    pub Driver: String,
    pub EnableIPv6: bool,
    pub IPAM: IPAM,
    pub Internal: bool,
    pub Attachable: bool,
    pub Ingress: bool,
    /// Container name to NetworkContainer
    pub Containers: HashMap<String, NetworkContainer>,
    pub Options: HashMap<String, String>,
    pub Labels: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct IPAM {
    pub Driver: String,
    pub Config: Option<Vec<IPAMConfig>>,
    #[serde(deserialize_with = "format::null_to_default")]
    pub Options: HashMap<String, String>,
}

impl Default for IPAM {
    fn default() -> Self {
        IPAM {
            Driver: "default".to_string(),
            Config: None,
            Options: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[allow(non_snake_case)]
pub struct IPAMConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Subnet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub IPRange: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Gateway: Option<String>,
    #[serde(
        skip_serializing_if = "HashMap::is_empty",
        deserialize_with = "format::null_to_default",
        default
    )]
    /// This field is given by "macvlan" network.
    ///
    /// # Example
    /// When the docker command is executed like below:
    /// ```text
    /// docker network create -d macvlan .. --aux-address="my-router=172.16.86.1" ..
    /// ```
    /// The value will be equals to `HashMap::from([("my-router", "172.16.86.5")])`.
    pub AuxiliaryAddresses: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct NetworkContainer {
    /// Container name
    /// e.g. gifted_turing
    pub Name: String,
    pub EndpointID: String,
    pub MacAddress: String,
    pub IPv4Address: String,
    pub IPv6Address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, Default)]
pub struct ListNetworkFilters {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub driver: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub id: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub label: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub name: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<NetworkScope>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub r#type: Vec<NetworkType>,
}

impl ListNetworkFilters {
    pub fn is_empty(&self) -> bool {
        self.driver.is_empty()
            && self.id.is_empty()
            && self.label.is_empty()
            && self.name.is_empty()
            && self.scope.is_empty()
            && self.r#type.is_empty()
    }

    pub fn driver(&mut self, driver: Cow<str>) -> &mut Self {
        self.driver.push(driver.into_owned());
        self
    }

    pub fn id(&mut self, id: Cow<str>) -> &mut Self {
        self.id.push(id.into_owned());
        self
    }

    pub fn label(&mut self, label: Cow<str>) -> &mut Self {
        self.label.push(label.into_owned());
        self
    }

    pub fn name(&mut self, name: Cow<str>) -> &mut Self {
        self.name.push(name.into_owned());
        self
    }
    pub fn scope(&mut self, scope: NetworkScope) -> &mut Self {
        self.scope.push(scope);
        self
    }

    pub fn r#type(&mut self, r#type: NetworkType) -> &mut Self {
        self.r#type.push(r#type);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PruneNetworkFilters {
    pub until: Vec<i64>,
    pub label: LabelFilter,
    pub label_not: LabelFilter,
}

impl Default for PruneNetworkFilters {
    fn default() -> Self {
        Self {
            until: vec![],
            label: LabelFilter::new(),
            label_not: LabelFilter::new(),
        }
    }
}

impl PruneNetworkFilters {
    pub fn is_empty(&self) -> bool {
        self.until.is_empty() && self.label.is_empty() && self.label_not.is_empty()
    }

    pub fn until(&mut self, until: Vec<i64>) -> &mut Self {
        self.until = until;
        self
    }

    pub fn label(&mut self, label: LabelFilter) -> &mut Self {
        self.label = label;
        self
    }

    pub fn label_not(&mut self, label_not: LabelFilter) -> &mut Self {
        self.label_not = label_not;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelFilter(HashMap<String, Option<String>>);

impl LabelFilter {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn with(filters: &[(&str, Option<&str>)]) -> Self {
        let mut map = HashMap::new();
        for (k, v) in filters {
            map.insert((*k).to_owned(), (*v).map(ToOwned::to_owned));
        }
        Self(map)
    }

    pub fn key(&mut self, key: &str) -> &mut Self {
        self.0.insert(key.to_owned(), None);
        self
    }

    pub fn key_value(&mut self, key: &str, value: &str) -> &mut Self {
        self.0.insert(key.to_owned(), Some(value.to_owned()));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkScope {
    Swarm,
    Global,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkType {
    Custom,
    Builtin,
}

/// request body of /networks/create api
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkCreateOptions {
    pub name: String,
    pub check_duplicate: bool,
    pub driver: String,
    /// Restrict connections between containers to only those under the same network.
    /// Default `false`
    pub internal: bool,
    pub attachable: bool,
    pub ingress: bool,
    #[serde(rename = "IPAM")]
    pub ipam: IPAM,
    #[serde(rename = "EnableIPv6")]
    pub enable_ipv6: bool,
    pub options: HashMap<String, String>,
    pub labels: HashMap<String, String>,
}

/// Create network options
///
/// To create a network equivalent to the default bridge network,
/// set the options as follows:
///
/// ```
/// # use crate::dockworker::network::*;
/// # use std::net::Ipv4Addr;
/// let network_name = "sample-network";
/// let mut opt = NetworkCreateOptions::new(network_name);
/// opt.enable_icc()
///     .enable_ip_masquerade()
///     .host_binding_ipv4(Ipv4Addr::new(0, 0, 0, 0))
///     .bridge_name("docker0")
///     .driver_mtu(1500);
/// // let network = docker.create_network(&opt)?;
/// ```
impl NetworkCreateOptions {
    /// equivalent to `docker network create <name>`
    pub fn new(name: &str) -> Self {
        Self {
            attachable: false,
            check_duplicate: true,
            driver: "bridge".to_owned(),
            enable_ipv6: false,
            ipam: IPAM::default(),
            ingress: false,
            internal: false,
            labels: HashMap::new(),
            name: name.to_owned(),
            options: HashMap::new(),
        }
    }

    fn force_bridge_driver(&mut self) {
        if &self.driver != "bridge" {
            warn!("network driver is {} (!= bridge)", self.driver);
            warn!("driver is enforced to bridge");
            self.driver = "bridge".to_owned();
        }
    }

    /// bridge name to be used when creating the Linux bridge
    pub fn bridge_name(&mut self, name: &str) -> &mut Self {
        self.force_bridge_driver();
        self.options
            .insert("com.docker.network.bridge.name".to_owned(), name.to_owned());
        self
    }

    /// equivalent to `--ip-masq` of dockerd flag
    pub fn enable_ip_masquerade(&mut self) -> &mut Self {
        self.force_bridge_driver();
        self.options.insert(
            "com.docker.network.bridge.enable_ip_masquerade".to_owned(),
            "true".to_owned(),
        );
        self
    }

    /// equivalent to `--icc` of dockerd flag
    pub fn enable_icc(&mut self) -> &mut Self {
        self.force_bridge_driver();
        self.options.insert(
            "com.docker.network.bridge.enable_icc".to_owned(),
            "true".to_owned(),
        );
        self
    }

    /// equivalent to `--ip` of dockerd flag
    pub fn host_binding_ipv4(&mut self, ipv4: Ipv4Addr) -> &mut Self {
        self.force_bridge_driver();
        self.options.insert(
            "com.docker.network.bridge.host_binding_ipv4".to_owned(),
            ipv4.to_string(),
        );
        self
    }

    /// equivalent to `--mtu` option
    pub fn driver_mtu(&mut self, mtu: u16) -> &mut Self {
        self.force_bridge_driver();
        self.options
            .insert("com.docker.network.driver.mtu".to_owned(), mtu.to_string());
        self
    }

    pub fn label(&mut self, key: &str, value: &str) -> &mut Self {
        self.labels.insert(key.to_owned(), value.to_owned());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct CreateNetworkResponse {
    pub Id: String,
    pub Warning: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PruneNetworkResponse {
    #[serde(
        serialize_with = "format::vec_to_null",
        deserialize_with = "format::null_to_default"
    )]
    pub networks_deleted: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[derive(Default)]
pub struct EndpointConfig {
    pub IPAMConfig: Option<EndpointIPAMConfig>,
    pub Links: Option<Vec<String>>,
    pub Aliases: Option<Vec<String>>,
    pub NetworkID: String,
    pub EndpointID: String,
    pub Gateway: String,
    pub IPAddress: String,
    pub IPPrefixLen: i64,
    pub IPv6Gateway: String,
    pub GlobalIPv6Address: String,
    pub GlobalIPv6PrefixLen: i64,
    pub MacAddress: String,
    #[serde(
        serialize_with = "format::hashmap_to_null",
        deserialize_with = "format::null_to_default",
        default
    )]
    pub DriverOpts: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize)]
#[allow(non_snake_case)]
#[serde(default)]
pub struct EndpointIPAMConfig {
    pub IPv4Address: String,
    pub IPv6Address: String,
    pub LinkLocalIPs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct NetworkConnectOptions {
    /// The ID or name of the container to connect to the network
    pub Container: String,
    /// Configuration for a network endpoint
    pub EndpointConfig: EndpointConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct NetworkDisconnectOptions {
    /// The ID or name of the container to disconnect to the network
    pub Container: String,
    /// Force the container to disconnect from the network
    pub Force: bool,
}

mod format {
    use super::*;

    use serde::de::{DeserializeOwned, Deserializer};
    use serde::{ser::*, Deserialize, Serialize, Serializer};

    pub fn null_to_default<'de, D, T>(de: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: DeserializeOwned + Default,
    {
        let actual: Option<T> = Option::deserialize(de)?;
        Ok(actual.unwrap_or_default())
    }

    pub fn vec_to_null<T, S>(t: &Vec<T>, se: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        if t.is_empty() {
            se.serialize_none()
        } else {
            t.serialize(se)
        }
    }

    pub fn hashmap_to_null<T, S>(t: &HashMap<String, T>, se: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        if t.is_empty() {
            se.serialize_none()
        } else {
            t.serialize(se)
        }
    }

    impl Serialize for PruneNetworkFilters {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let count = [
                self.until.is_empty(),
                self.label.is_empty(),
                self.label_not.is_empty(),
            ]
            .iter()
            .filter(|x| **x)
            .count();

            let mut state = serializer.serialize_map(Some(count))?;
            if !self.until.is_empty() {
                state.serialize_entry("until", &UntilTimestamp(&self.until))?;
            }
            if !self.label.is_empty() {
                state.serialize_entry("label", &self.label)?;
            }
            if !self.label_not.is_empty() {
                state.serialize_entry("label!", &self.label_not)?;
            }
            state.end()
        }
    }

    impl Serialize for EndpointIPAMConfig {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if self.IPv4Address.is_empty()
                && self.IPv6Address.is_empty()
                && self.LinkLocalIPs.is_empty()
            {
                let map = serializer.serialize_map(Some(0))?;
                map.end()
            } else {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("IPv4Address", &self.IPv4Address)?;
                map.serialize_entry("IPv6Address", &self.IPv6Address)?;
                map.serialize_entry("LinkLocalIPs", &self.LinkLocalIPs)?;
                map.end()
            }
        }
    }

    #[derive(Debug, Clone)]
    struct UntilTimestamp<'a>(&'a Vec<i64>);

    impl<'a> Serialize for UntilTimestamp<'a> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut map = serializer.serialize_map(Some(self.0.len()))?;
            for tm in self.0 {
                map.serialize_entry(&tm.to_string(), &true)?;
            }
            map.end()
        }
    }

    impl Serialize for LabelFilter {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut map = serializer.serialize_map(None)?;
            for (k, v) in &self.0 {
                let key = match v {
                    Some(v) => format!("{k}={v}"),
                    None => k.to_string(),
                };
                map.serialize_entry(&key, &true)?;
            }
            map.end()
        }
    }
}
