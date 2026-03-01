use std::time::Duration;

use tokio::sync::watch;

use crate::{
    agent::{AgentHandle, Event},
    config::{HealthCheck, HealthCheckKind},
    state::ServiceHealth,
};

pub async fn run_healthcheck(
    name: String,
    hc: HealthCheck,
    tx: watch::Sender<ServiceHealth>,
    agent_handle: AgentHandle,
) {
    if let Some(grace) = hc.startup_grace_s {
        tokio::time::sleep(Duration::from_secs(grace)).await;
    }

    let mut prev_status = ServiceHealth::Unknown;

    loop {
        let ok = match hc.kind() {
            Ok(HealthCheckKind::Http(url)) => check_http(url, hc.timeout_ms).await,
            Ok(HealthCheckKind::Tcp(addr)) => check_tcp(addr, hc.timeout_ms).await,
            Err(_) => break,
        };

        let current_status = if ok {
            println!("[{}] healthy", name);
            ServiceHealth::Healthy
        } else {
            println!("[{}] healthcheck failed", name);
            ServiceHealth::Unhealthy
        };

        let _ = tx.send(current_status.clone());
        if prev_status != current_status {
            agent_handle.emit(Event::HealthChanged {
                service: name.clone(),
                healthy: (current_status == ServiceHealth::Healthy),
            });
            prev_status = current_status;
        }

        tokio::time::sleep(Duration::from_millis(hc.interval_ms)).await;
    }
}

async fn check_http(url: &str, timeout_ms: u64) -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .unwrap();

    client
        .get(url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

async fn check_tcp(addr: &str, timeout_ms: u64) -> bool {
    tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        tokio::net::TcpStream::connect(addr),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false)
}
