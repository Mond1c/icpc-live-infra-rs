mod agent;
mod config;
mod healthcheck;
mod state;
mod supervisor;

#[tokio::main]
async fn main() {
    tokio::fs::create_dir("logs").await.unwrap();
    let broadcast = config::Broadcast::load("examples/simple.toml").expect("can't parse config");

    let names: Vec<String> = broadcast.service.iter().map(|s| s.name.clone()).collect();

    let health_map = state::build_health_map(&names);

    let mut handles = vec![];

    let agent_handle = agent::AgentHandle::new();

    tokio::spawn(agent::run(agent_handle.clone(), 7700));

    for service in broadcast.service {
        let handle = tokio::spawn(supervisor::run_service(
            service,
            health_map.clone(),
            agent_handle.clone(),
        ));
        handles.push(handle);
    }

    for h in handles {
        h.await.unwrap();
    }
}
