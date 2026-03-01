use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::{net::TcpListener, sync::broadcast};
use tokio_tungstenite::accept_async;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum Event {
    ServiceStarted { service: String },
    ServiceExited { service: String, code: Option<i32> },
    HealthChanged { service: String, healthy: bool },
    LogLine { service: String, line: String },
}

#[derive(Clone)]
pub struct AgentHandle {
    pub tx: broadcast::Sender<Event>,
}

impl AgentHandle {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { tx }
    }

    pub fn emit(&self, event: Event) {
        let _ = self.tx.send(event);
    }
}

pub async fn run(handle: AgentHandle, port: u16) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    println!("agent listening on :{}", port);

    loop {
        let (stream, addr) = listener.accept().await?;
        let mut rx = handle.tx.subscribe();
        println!("ctl connected from {}", addr);

        tokio::spawn(async move {
            let mut ws = accept_async(stream).await.unwrap();

            while let Ok(event) = rx.recv().await {
                let json = serde_json::to_string(&event).unwrap();
                if ws.send(json.into()).await.is_err() {
                    break;
                }
            }
        });
    }
}
