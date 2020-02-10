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
    pub Config: Vec<HashMap<String, String>>,
    #[serde(deserialize_with = "format::null_to_default")]
    pub Options: HashMap<String, String>,
}

impl Default for IPAM {
    fn default() -> Self {
        IPAM {
            Driver: "default".to_string(),
            Config: vec![],
            Options: HashMap::new(),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
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

impl Default for ListNetworkFilters {
    fn default() -> Self {
        ListNetworkFilters {
            driver: vec![],
            id: vec![],
            label: vec![],
            name: vec![],
            scope: vec![],
            r#type: vec![],
        }
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

impl NetworkCreateOptions {
    pub fn default_bridge_option() -> HashMap<String, String> {
        vec![
            (
                "com.docker.network.bridge.name".to_owned(),
                "bridge".to_owned(),
            ),
            (
                "com.docker.network.bridge.enable_ip_masquerade".to_owned(),
                "true".to_owned(),
            ),
            (
                "com.docker.network.bridge.enable_icc".to_owned(),
                "true".to_owned(),
            ),
            (
                "com.docker.network.bridge.host_binding_ipv4".to_owned(),
                "0.0.0.0".to_owned(),
            ),
            (
                "com.docker.network.driver.mtu".to_owned(),
                "1500".to_owned(),
            ),
        ]
        .into_iter()
        .collect()
    }

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
            options: Self::default_bridge_option(),
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
    pub networks_deleted: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct EndpointConfig {
    pub IPAMConfig: Option<EndpointIPAMConfig>,
    pub Links: Vec<String>,
    pub Aliases: Vec<String>,
    pub NetworkID: String,
    pub EndpointID: String,
    pub Gateway: String,
    pub IPAddress: String,
    pub IPPrefixLen: i64,
    pub IPv6Gateway: String,
    pub GlobalIPv6Address: String,
    pub GlobalIPv6PrefixLen: i64,
    pub MacAddress: String,
    #[serde(serialize_with = "format::hashmap_to_null")]
    pub DriverOpts: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct EndpointIPAMConfig {
    pub IPv4Address: String,
    pub IPv6Address: String,
    #[serde(deserialize_with = "format::null_to_default")]
    #[serde(serialize_with = "format::vec_to_null")]
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
    use serde::{Deserialize, Serialize, Serializer};

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
}
