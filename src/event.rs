use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[allow(non_snake_case)]
pub struct EventActor {
    pub ID: String,
    pub Attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(non_snake_case)]
pub struct EventResponse {
    pub Type: String,
    pub Action: String,
    pub Actor: EventActor,
    pub time: u64,
    pub timeNano: u64,
}
