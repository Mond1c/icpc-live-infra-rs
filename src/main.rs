mod config;
mod supervisor;

#[tokio::main]
async fn main() {
    let broadcast = config::Broadcast::load("examples/simple.toml").expect("can't parse config");
    println!("{:#?}", broadcast);

    let mut handles = vec![];

    for service in broadcast.service {
        let handle = tokio::spawn(supervisor::run_service(service));
        handles.push(handle);
    }

    for h in handles {
        h.await.unwrap();
    }
}
