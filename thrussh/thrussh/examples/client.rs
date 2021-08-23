extern crate env_logger;
extern crate futures;
extern crate thrussh;
extern crate thrussh_keys;
extern crate tokio;
use anyhow::Context;
use std::sync::Arc;
use thrussh::*;
use thrussh_keys::*;
use async_trait::async_trait;

struct Client {}

#[async_trait]
impl client::Handler for Client {
    type Error = thrussh::Error;

    async fn check_server_key(self, server_public_key: &key::PublicKey) -> Result<(Self, bool), Self::Error> {
        println!("check_server_key: {:?}", server_public_key);
        Ok((self, true))
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = thrussh::client::Config::default();
    let config = Arc::new(config);
    let sh = Client {};

    let mut agent = thrussh_keys::agent::client::AgentClient::connect_env()
        .await
        .unwrap();
    let mut identities = agent.request_identities().await.unwrap();
    let mut session = thrussh::client::connect(config, "127.0.0.1:2200", sh)
        .await
        .unwrap();
    let (_, auth_res) = session
        .authenticate_future("pe", identities.pop().unwrap(), agent)
        .await;
    let auth_res = auth_res.unwrap();
    println!("=== auth: {}", auth_res);
    let mut channel = session
        .channel_open_direct_tcpip("localhost", 8000, "localhost", 3333)
        .await
        .unwrap();
    // let mut channel = session.channel_open_session().await.unwrap();
    println!("=== after open channel");
    let data = b"GET /les_affames.mkv HTTP/1.1\nUser-Agent: curl/7.68.0\nAccept: */*\nConnection: close\n\n";
    channel.data(&data[..]).await.unwrap();
    let mut f = std::fs::File::create("les_affames.mkv").unwrap();
    while let Some(msg) = channel.wait().await {
        use std::io::Write;
        match msg {
            thrussh::ChannelMsg::Data { ref data } => {
                f.write_all(&data).unwrap();
            }
            thrussh::ChannelMsg::Eof => {
                f.flush().unwrap();
                break;
            }
            _ => {}
        }
    }
    session
        .disconnect(Disconnect::ByApplication, "", "English")
        .await
        .unwrap();
    let res = session.await.context("session await");
    println!("{:#?}", res);
}
