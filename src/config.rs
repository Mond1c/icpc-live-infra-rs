use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Broadcast {
    pub topology: Topology,
    pub node: Vec<Node>,
    pub service: Vec<Service>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Topology {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub name: String,
    pub host: String,
    pub agent_port: u16,
    pub services: Vec<String>,
    pub deploy_user: Option<String>,
    pub deploy_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Service {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub restart: String,
    pub healthcheck: Option<HealthCheck>,
    pub depends_on: Option<Vec<Dependency>>,
    pub deploy: Option<ServiceDeploy>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceDeploy {
    pub files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthCheck {
    pub http: Option<String>,
    pub tcp: Option<String>,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub startup_grace_s: Option<u64>,
}

pub enum HealthCheckKind<'a> {
    Http(&'a str),
    Tcp(&'a str),
}

impl HealthCheck {
    pub fn kind(&self) -> anyhow::Result<HealthCheckKind<'_>> {
        match (&self.http, &self.tcp) {
            (Some(url), None) => Ok(HealthCheckKind::Http(url)),
            (None, Some(addr)) => Ok(HealthCheckKind::Tcp(addr)),
            (Some(_), Some(_)) => anyhow::bail!("healthcheck must be http or tcp, not both"),
            (None, None) => anyhow::bail!("healthcheck must be http or tcp, indicate one of them"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Dependency {
    pub service: String,
    pub node: String,
}

impl Broadcast {
    pub fn load(path: &str) -> anyhow::Result<Broadcast> {
        let content = std::fs::read_to_string(path)?;
        let broadcast: Broadcast = toml::from_str(&content)?;
        Ok(broadcast)
    }
}
