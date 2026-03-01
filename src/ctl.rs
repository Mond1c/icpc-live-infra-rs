use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::StreamExt;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;

use crate::agent::Event;

#[derive(Debug, Clone)]
pub struct ServiceState {
    pub healthy: Option<bool>,
    pub running: bool,
    pub last_exit_code: Option<i32>,
}

impl Default for ServiceState {
    fn default() -> Self {
        ServiceState {
            healthy: None,
            running: false,
            last_exit_code: None,
        }
    }
}

pub type ClusterState = Arc<Mutex<HashMap<String, ServiceState>>>;

pub fn new_cluster_state() -> ClusterState {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn watch_node(node_name: String, host: String, port: u16, state: ClusterState) {
    let url = format!("ws://{}:{}", host, port);

    loop {
        println!("[ctl] connecting to {} at {}...", node_name, url);

        match connect_async(&url).await {
            Ok((mut ws, _)) => {
                println!("[ctl] connected to {}", node_name);

                while let Some(msg) = ws.next().await {
                    match msg {
                        Ok(m) if m.is_text() => {
                            let text = m.into_text().unwrap();
                            if let Ok(event) = serde_json::from_str::<Event>(&text) {
                                handle_event(&node_name, event, &state).await;
                            }
                        }
                        Err(e) => {
                            println!("[ctl] {} disconnected: {}", node_name, e);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                println!("[ctl] can't connect to {}: {}", node_name, e);
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn handle_event(node: &str, event: Event, state: &ClusterState) {
    let mut map = state.lock().await;

    match event {
        Event::ServiceStarted { service } => {
            let key = format!("{}/{}", node, service);
            let entry = map.entry(key.clone()).or_default();
            entry.running = true;
            println!("[ctl] {} started", key);
        }
        Event::ServiceExited { service, code } => {
            let key = format!("{}/{}", node, service);
            let entry = map.entry(key.clone()).or_default();
            entry.running = false;
            entry.last_exit_code = code;
            println!("[ctl] {} exited with {:?}", key, code);
        }
        Event::HealthChanged { service, healthy } => {
            let key = format!("{}/{}", node, service);
            let entry = map.entry(key.clone()).or_default();
            entry.healthy = Some(healthy);
            println!("[ctl] {} healthy={}", key, healthy);
        }
        _ => {}
    }
}
