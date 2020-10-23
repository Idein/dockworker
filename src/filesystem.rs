use serde::{Deserialize, Serialize};

/// response of /containers/{id}/changes
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct FilesystemChange {
    pub Path: String,
    pub Kind: u8,
}

/// content of X-Docker-Container-Path-Stat header
/// acquired from HEAD /containers/{id}/archive
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct XDockerContainerPathStat {
    pub name: String,
    pub size: u64,
    pub mode: u64,
    pub mtime: String,
    pub linkTarget: String,
}
