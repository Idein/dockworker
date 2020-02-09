use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};

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
    pub Containers: HashMap<String, NetworkContainer>,
    pub Options: HashMap<String, String>,
    pub Labels: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct IPAM {
    pub Driver: String,
    pub Config: Vec<HashMap<String, String>>,
    #[serde(serialize_with = "format::empty_to_null")]
    #[serde(deserialize_with = "format::null_to_default")]
    pub Options: Vec<HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct NetworkContainer {
    pub Name: String,
    pub EndpointID: String,
    pub MacAddress: String,
    pub IPv4Address: Ipv4Addr,
    pub IPv6Address: Ipv6Addr,
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
#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize)]
#[allow(non_snake_case)]
pub struct CreateNetworkResponse {
    pub Id: String,
    pub Warning: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PruneNetworkResponse {
    pub networks_deleted: Vec<String>,
}

pub mod format {
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

    pub fn empty_to_null<T, S>(t: &Vec<T>, se: S) -> Result<S::Ok, S::Error>
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
