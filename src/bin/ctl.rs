use icpc_live_infra_rs::{config, ctl};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config_path = args.get(1).map(|s| s.as_str()).unwrap_or("broadcast.toml");

    let broadcast = config::Broadcast::load(config_path).expect("can't parse config");

    let cluster_state = ctl::new_cluster_state();

    let mut handles = vec![];
    for node in broadcast.node {
        let node_services: Vec<config::Service> = broadcast
            .service
            .iter()
            .filter(|s| node.services.contains(&s.name))
            .cloned()
            .collect();

        let h = tokio::spawn(ctl::watch_node(
            node.name,
            node.host,
            node.agent_port,
            cluster_state.clone(),
            node_services,
        ));
        handles.push(h);
    }

    for h in handles {
        h.await.unwrap();
    }
}
