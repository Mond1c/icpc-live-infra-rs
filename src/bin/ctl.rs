use icpc_live_infra_rs::{
    config::{self, Broadcast},
    ctl, deploy,
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config_path = args.get(1).map(|s| s.as_str()).unwrap_or("broadcast.toml");
    let broadcast = config::Broadcast::load(config_path).expect("can't parse config");

    match args.get(2).map(|s| s.as_str()) {
        Some("deploy") => {
            let target = args.get(3).map(|s| s.as_str());
            run_deploy(&broadcast, target).await;
        }
        _ => {
            run_watch(&broadcast).await;
        }
    }
}

async fn run_deploy(broadcast: &Broadcast, target: Option<&str>) {
    let nodes: Vec<&config::Node> = broadcast
        .node
        .iter()
        .filter(|n| target.map_or(true, |t| n.name == t))
        .collect();

    for node in nodes {
        let services: Vec<config::Service> = broadcast
            .service
            .iter()
            .filter(|s| node.services.contains(&s.name))
            .cloned()
            .collect();

        if let Err(e) = deploy::deploy_node(node, &services, broadcast).await {
            eprintln!("[deploy] {} failed: {}", node.name, e);
        }
    }
}

async fn run_watch(broadcast: &Broadcast) {
    let cluster_state = ctl::new_cluster_state();
    let mut handles = vec![];

    for node in &broadcast.node {
        let node_services: Vec<config::Service> = broadcast
            .service
            .iter()
            .filter(|s| node.services.contains(&s.name))
            .cloned()
            .collect();

        let h = tokio::spawn(ctl::watch_node(
            node.name.clone(),
            node.host.clone(),
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
