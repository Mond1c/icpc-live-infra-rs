use std::{process::Stdio, time::Duration};

use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

use crate::config::Service;

pub async fn run_service(service: Service) {
    loop {
        println!("[{}] starting...", service.name);

        let child = Command::new(&service.command)
            .args(&service.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                println!("[{}] failed to spawn: {}", service.name, e);
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

        let status = child.wait().await;

        match status {
            Ok(s) => println!("[{}] exited with {}", service.name, s),
            Err(e) => println!("[{}] failed to spawn: {}", service.name, e),
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
