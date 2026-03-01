use std::{process::Stdio, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

use crate::config::Service;

pub async fn run_service(service: Service) {
    loop {
        println!("[{}] starting...", service.name);

        let mut child = Command::new(&service.command)
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

        if let Some(stdout) = child.stdout.take() {
            let name = service.name.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    println!("[{name}] {line}");
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let name = service.name.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    eprintln!("[{name}] {line}");
                }
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
