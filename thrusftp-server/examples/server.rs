use thrusftp_server::SftpServer;
use thrusftp_server::thrussh::start_server;
use thrusftp_fs_local::LocalFs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    start_server(SftpServer::new(LocalFs)).await;
    Ok(())
}
