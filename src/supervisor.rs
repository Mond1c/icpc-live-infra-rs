use std::{process::Stdio, time::Duration};

use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::watch,
};

use crate::{
    agent::{AgentHandle, Event},
    config::Service,
    healthcheck,
    state::{HealthMap, ServiceHealth},
};

#[derive(Debug, Clone, PartialEq)]
pub enum SupervisorCommand {
    Restart,
    Stop,
}

pub async fn run_service(
    service: Service,
    health_map: HealthMap,
    agent_handle: AgentHandle,
    mut cmd_rx: watch::Receiver<Option<SupervisorCommand>>,
) {
    if let Some(deps) = &service.depends_on {
        for dep in deps {
            if let Some(tx) = health_map.get(&dep.service) {
                let mut rx = tx.subscribe();
                println!("[{}] waiting for {}...", service.name, dep.service);
                rx.wait_for(|h| *h == ServiceHealth::Healthy).await.unwrap();
                println!("[{}] dependency {} is healthy", service.name, dep.service);
            }
        }
    }

    loop {
        println!("[{}] starting...", service.name);
        agent_handle.emit(Event::ServiceStarted {
            service: service.name.clone(),
        });

        let child = Command::new(&service.command)
            .args(&service.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                println!("[{}] failed to spawn: {}", service.name, e);
                agent_handle.emit(Event::ServiceExited {
                    service: service.name.clone(),
                    code: Some(-1),
                });
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        let log_path = format!("logs/{}.log", service.name);

        if let Some(stdout) = child.stdout.take() {
            let path = log_path.clone();
            tokio::spawn(async move {
                pipe_to_file(stdout, &path).await;
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let path = log_path.clone();
            tokio::spawn(async move {
                pipe_to_file(stderr, &path).await;
            });
        }

        if let Some(hc) = service.healthcheck.clone() {
            let name = service.name.clone();
            let tx = health_map.get(&name).unwrap().clone();
            tokio::spawn(healthcheck::run_healthcheck(
                name,
                hc,
                tx,
                agent_handle.clone(),
            ));
        }

        tokio::select! {
            status = child.wait() => {
                match status {
                    Ok(s) => println!("[{}] exited with {}", service.name, s),
                    Err(e) => {
                        println!("[{}] failed to spawn: {}", service.name, e);
                        agent_handle.emit(Event::ServiceExited {
                            service: service.name.clone(),
                            code: Some(-2),
                        });
                    }
                }
            }
            Ok(()) = cmd_rx.changed() => {
                let cmd = cmd_rx.borrow().clone();
                match cmd {
                    Some(SupervisorCommand::Restart) => {
                        println!("[{}] restart requested", service.name);
                        let _ = child.kill().await;
                    }
                    Some(SupervisorCommand::Stop) => {
                        let _ = child.kill().await;
                        return;
                    }
                    None => {}
                }
            }
        }

        match service.restart.as_str() {
            "always" => {
                println!("[{}] restarting in 2s...", service.name);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            _ => return,
        }
    }
}

async fn pipe_to_file(stream: impl tokio::io::AsyncRead + Unpin, path: &str) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
        .expect("can't open log file");

    let mut lines = BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let entry = format!("[{}] {}\n", chrono::Utc::now().to_rfc3339(), line);
        let _ = file.write_all(entry.as_bytes()).await;
    }
}
