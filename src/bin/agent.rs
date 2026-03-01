use icpc_live_infra_rs::{agent, config, state, supervisor};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config_path = args.get(1).map(|s| s.as_str()).unwrap_or("broadcast.toml");

    tokio::fs::create_dir_all("logs").await.unwrap();

    let broadcast = config::Broadcast::load(config_path).expect("can't parse config");

    let port: u16 = args.get(2).and_then(|p| p.parse().ok()).unwrap_or(7700);

    let names: Vec<String> = broadcast.service.iter().map(|s| s.name.clone()).collect();
    let health_map = state::build_health_map(&names);
    let agent_handle = agent::AgentHandle::new();

    tokio::spawn(agent::run(agent_handle.clone(), port));

    let mut handles = vec![];
    for service in broadcast.service {
        let h = tokio::spawn(supervisor::run_service(
            service,
            health_map.clone(),
            agent_handle.clone(),
        ));
        handles.push(h);
    }

    for h in handles {
        h.await.unwrap();
    }
}
