use std::sync::Arc;

use russh::{client, keys::load_secret_key};
use russh_sftp::client::SftpSession;
use tokio::fs;

use crate::config::{Broadcast, Node, Service};

pub async fn deploy_node(
    node: &Node,
    services: &[Service],
    broadcast: &Broadcast,
) -> anyhow::Result<()> {
    let user = node.deploy_user.as_deref().unwrap_or("root");
    let deploy_path = node.deploy_path.as_deref().unwrap_or("/opt/icpc");

    println!("[deploy] connecting to {}@{}...", user, node.host);

    let config = Arc::new(russh::client::Config::default());
    let mut session = client::connect(config, (node.host.as_str(), 22), Handler).await?;

    session
        .authenticate_publickey(user, load_key().await?)
        .await?;

    println!("[deploy] connected, uploading files...");

    let channel = session.channel_open_session().await?;
    channel.request_subsystem(true, "sftp").await?;
    let sftp = SftpSession::new(channel.into_stream()).await?;

    sftp.create_dir(deploy_path).await.ok();

    for service in services {
        if let Some(deploy) = &service.deploy {
            for file in &deploy.files {
                upload(&sftp, file, deploy_path).await?;
            }
        }
    }

    upload(&sftp, "target/release/agent", deploy_path).await?;

    println!("[deploy] files uploaded, restarting agent....");

    channel
        .exec(
            true,
            format!(
                "pkill -f 'agent' || true && cd {} && nohup ./agent {} &",
                deploy_path, node.agent_port,
            ),
        )
        .await?;

    println!("[deploy] {} done", node.name);
    Ok(())
}

async fn upload(sftp: &SftpSession, local: &str, remote_dir: &str) -> anyhow::Result<()> {
    let filename = std::path::Path::new(local)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    let remote_path = format!("{}/{}", remote_dir, filename);

    println!("[deploy]    {} -> {}", local, remote_path);

    let data = fs::read(local).await?;
    let mut remote = sftp.create(&remote_path).await?;

    use tokio::io::AsyncWriteExt;
    remote.write_all(&data).await;
    Ok(())
}

async fn load_key() -> anyhow::Result<Arc<russh::keys::key::KeyPair>> {
    let home = std::env::var("HOME")?;
    let key = load_secret_key(format!("{}/.ssh/id_ed25519", home), None)?;

    Ok(Arc::new(key))
}

struct Handler;

#[async_trait::async_trait]
impl client::Handler for Handler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _key: &russh::keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
