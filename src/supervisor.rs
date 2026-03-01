use std::time::Duration;

use tokio::process::Command;

use crate::config::Service;

pub async fn run_service(service: Service) {
    loop {
        println!("[{}] starting...", service.name);

        let status = Command::new(&service.command)
            .args(&service.args)
            .status()
            .await;

        match status {
            Ok(s) => println!("[{}] exited with {}", service.name, s),
            Err(e) => println!("[{}] failed to spawn: {}", service.name, e),
        }

        match service.restart.as_str() {
            "always" => {
                println!("[{}] restarting in 2s...", service.name);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            "never" => {
                println!("[{}] not restarting", service.name);
                return;
            }
            _ => return,
        }
    }
}
