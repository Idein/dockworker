use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct Version {
    pub Version: String,
    pub ApiVersion: String,
    #[serde(default = "String::default")]
    pub MinAPIVersion: String,
    pub GitCommit: String,
    pub GoVersion: String,
    pub Os: String,
    pub Arch: String,
    pub KernelVersion: String,
    pub Experimental: Option<bool>,
    pub BuildTime: Option<String>,
}
