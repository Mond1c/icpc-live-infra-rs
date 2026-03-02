use std::collections::HashMap;

use futures_util::{SinkExt, StreamExt};
use icpc_live_infra_rs::{
    agent,
    protocol::{AgentMessage, CtlMessage},
    state,
    supervisor::{self, SupervisorCommand},
};
use tokio::{net::TcpListener, sync::watch};
use tokio_tungstenite::accept_async;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args.get(1).and_then(|p| p.parse().ok()).unwrap_or(7700);

    tokio::fs::create_dir_all("logs").await.unwrap();

    let listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    println!("[agent] listening on :{port}");

    while let Ok((stream, addr)) = listener.accept().await {
        println!("[agent] ctl connected from {addr}");
        handle_ctl(stream).await;
        println!("[agent] ctl disconnected");
    }
}

async fn handle_ctl(stream: tokio::net::TcpStream) {
    let mut ws = accept_async(stream).await.unwrap();
    let agent_handle = agent::AgentHandle::new();
    let mut cmd_txs: HashMap<String, watch::Sender<Option<SupervisorCommand>>> = HashMap::new();

    while let Some(Ok(msg)) = ws.next().await {
        if !msg.is_text() {
            continue;
        }

        let text = msg.into_text().unwrap();
        match serde_json::from_str::<CtlMessage>(&text) {
            Ok(CtlMessage::Deploy { services }) => {
                println!("[agent] received deploy: {} services", services.len());

                let names: Vec<String> = services.iter().map(|s| s.name.clone()).collect();
                let health_map = state::build_health_map(&names);

                for service in services {
                    let (cmd_tx, cmd_rx) = watch::channel(None);
                    cmd_txs.insert(service.name.clone(), cmd_tx);
                    tokio::spawn(supervisor::run_service(
                        service,
                        health_map.clone(),
                        agent_handle.clone(),
                        cmd_rx,
                    ));
                }

                let ready = serde_json::to_string(&AgentMessage::Ready {
                    node: "agent".into(),
                })
                .unwrap();
                let _ = ws.send(ready.into()).await;
                break;
            }
            Ok(other) => {
                println!("[agent] unexpected message before deploy: {:?}", other);
            }
            Err(e) => {
                println!("[agent] bad message: {}", e);
            }
        }
    }

    let mut rx = agent_handle.tx.subscribe();
    loop {
        tokio::select! {
            Some(Ok(msg)) = ws.next() => {
                if !msg.is_text() { continue; }
                let text = msg.into_text().unwrap();
                match serde_json::from_str::<CtlMessage>(&text) {
                    Ok(CtlMessage::Restart { service }) => {
                        if let Some(tx) = cmd_txs.get(&service) {
                            let _ = tx.send(Some(SupervisorCommand::Restart));
                        } else {
                            println!("[agent] unknown service: {}", service);
                        }
                    }
                    Ok(CtlMessage::Stop { service }) => {
                        if let Some(tx) = cmd_txs.get(&service) {
                            let _ = tx.send(Some(SupervisorCommand::Stop));
                        }
                    }
                    _ => {}
                }
            }
            Ok(event) = rx.recv() => {
                let json = serde_json::to_string(&event).unwrap();
                if ws.send(json.into()).await.is_err() {
                    break;
                }
            }
        }
    }
}
