///! Response from Dockerd
///!

use serde_json::value as json;

#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct ProgressDetail {
    pub current: u64,
    pub total: u64,
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Progress {
    /// image tag or hash of image layer
    pub id: String,
    /// progress bar
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<String>,
    pub progressDetail: Option<ProgressDetail>,
    /// message or auxiliary info
    pub status: String,
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct Status {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub status: String,
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub message: String,
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Error {
    pub error: String,
    pub errorDetail: ErrorDetail,
}

/// Response of /images/create or other API
///
/// ## NOTE
/// Structure of this type is not documented officialy.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    Progress(Progress),
    Status(Status),
    Error(Error),
    /// unknown response
    Unknown(json::Value),
}
