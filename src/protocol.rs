use serde::{Deserialize, Serialize};

use crate::config::Service;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum CtlMessage {
    Deploy { services: Vec<Service> },
    Start { service: String },
    Stop { service: String },
    Restart { service: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum AgentMessage {
    Ready { node: String },
    ServiceStarted { service: String },
    ServiceExited { service: String, code: Option<i32> },
    HealthChanged { service: String, healhy: bool },
}
