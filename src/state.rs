use std::{collections::HashMap, sync::Arc};

use tokio::sync::watch;

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceHealth {
    Unknown,
    Healthy,
    Unhealthy,
}

pub type HealthMap = Arc<HashMap<String, watch::Sender<ServiceHealth>>>;

pub fn build_health_map(service_names: &[String]) -> HealthMap {
    let mut map = HashMap::new();
    for name in service_names {
        let (tx, _) = watch::channel(ServiceHealth::Unknown);
        map.insert(name.clone(), tx);
    }
    Arc::new(map)
}
