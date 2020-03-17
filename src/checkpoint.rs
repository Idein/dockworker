#[cfg(feature = "experimental")]
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct Checkpoint {
    pub Name: String,
}

#[cfg(feature = "experimental")]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CheckpointCreateOptions {
    pub checkpoint_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    // None -> set by docker to /var/lib/docker/containers/{containerid}/checkpoints
    pub checkpoint_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // None -> set by docker to false, container keeps running by default
    pub exit: Option<bool>,
}

#[cfg(feature = "experimental")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CheckpointDeleteOptions {
    pub checkpoint_id: String,
    // None -> set by docker to /var/lib/docker/containers/{containerid}/checkpoints
    pub checkpoint_dir: Option<String>,
}
