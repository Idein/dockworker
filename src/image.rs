use serde::de::{Deserializer, DeserializeOwned};
use serde::Deserialize;

fn null_to_default<'de, D, T>(de: D) -> Result<T, D::Error>
    where D: Deserializer<'de>,
          T: DeserializeOwned + Default
{
    let actual : Option<T> = Option::deserialize(de)?;
    Ok(actual.unwrap_or_default())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Image {
    pub Id: String,
    pub ParentId: String,
    #[serde(deserialize_with = "null_to_default")]
    pub RepoTags: Vec<String>,
    #[serde(deserialize_with = "null_to_default", default = "Vec::default")]
    pub RepoDigests: Vec<String>,
    pub Created: u64,
    pub Size: i64,
    #[serde(default = "i64::default")]
    pub SharedSize: i64,
    pub VirtualSize: i64,
    #[serde(default = "i64::default")]
    pub Containers: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageStatus {
    pub status: Option<String>,
    pub error: Option<String>
}
