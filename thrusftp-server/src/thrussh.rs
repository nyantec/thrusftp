use thrussh::*;
use thrussh::server::Session;
use async_trait::async_trait;
use std::convert::TryInto;
use std::sync::Arc;
use tokio::io::AsyncReadExt;

use crate::SftpServer;
use thrusftp_protocol::types::*;
use thrusftp_protocol::Fs;
use thrusftp_protocol::parse::{Serialize, Deserialize};
use anyhow::Result;

pub async fn start_server<T: 'static + Fs + Send + Sync>(server: Arc<SftpServer<T>>) {
    let mut config = thrussh::server::Config::default();
    config.connection_timeout = Some(std::time::Duration::from_secs(300));
    config.auth_rejection_time = std::time::Duration::from_millis(300);
    config.keys.push(thrussh_keys::key::KeyPair::generate_ed25519().unwrap());
    let server = Server { server };
    thrussh::server::run(Arc::new(config), "0.0.0.0:2222", server).await.unwrap();
}

struct Server<T: Fs + Send + Sync> {
    server: Arc<SftpServer<T>>,
}

#[async_trait]
impl<T: Fs + Send + Sync> thrussh::server::Server for Server<T> {
    type Handler = Client<T>;
    async fn new(&mut self, _: Option<std::net::SocketAddr>) -> Client<T> {
        Client {
            recv_buf: Vec::new(),
            handle: self.server.clone().create_client_handle("client").await,
            server: self.server.clone(),
        }
    }
}

struct Client<T: Fs + Send + Sync> {
    recv_buf: Vec<u8>,
    handle: String,
    server: Arc<SftpServer<T>>,
}

#[async_trait]
impl<T: Fs + Send + Sync> thrussh::server::Handler for Client<T> {
    type Error = anyhow::Error;

    async fn shell_request(self, channel: ChannelId, mut session: Session) -> Result<(Self, Session)> {
        session.channel_success(channel);
        session.data(channel, CryptoVec::from_slice(b"Only SFTP allowed, bye\n"));
        session.flush()?;
        session.close(channel);
        Ok((self, session))
    }

    async fn subsystem_request(self, channel: ChannelId, name: &str, mut session: Session) -> Result<(Self, Session)> {
        match name {
            "sftp" => session.channel_success(channel),
            _ => {
                session.channel_failure(channel);
                session.close(channel);
            },
        };
        Ok((self, session))
    }

    async fn auth_publickey(self, _: &str, _: &thrussh_keys::key::PublicKey) -> Result<(Self, thrussh::server::Auth)> {
        Ok((self, thrussh::server::Auth::Accept))
    }

    async fn data(mut self, channel: ChannelId, mut data: &[u8], mut session: Session) -> Result<(Self, Session)> {
        while data.len() > 0 {
            if self.recv_buf.len() < 4 {
                let read_len = data.take((4 - self.recv_buf.len()) as u64).read_to_end(&mut self.recv_buf).await.unwrap();
                data = &data[read_len..];
            }

            if self.recv_buf.len() >= 4 {
                let len = u32::from_be_bytes(self.recv_buf[..4].try_into().unwrap()) as usize;
                let needed = (len + 4) - self.recv_buf.len();

                let read_len = data.take(needed as u64).read_to_end(&mut self.recv_buf).await.unwrap();
                data = &data[read_len..];
                if read_len == needed {
                    let recv_buf = &self.recv_buf.as_slice();
                    let packet = SftpClientPacket::deserialize(&mut &recv_buf[4..]).unwrap();
                    self.recv_buf.clear();

                    let resp = self.server.clone().process(&self.handle, packet).await;

                    let mut resp_buf = Vec::new();
                    let mut resp_bytes = resp.serialize().unwrap();
                    resp_buf.append(&mut u32::serialize(&(resp_bytes.len() as u32))?);
                    resp_buf.append(&mut resp_bytes);
                    session.data(channel, CryptoVec::from_slice(&resp_buf));
                }
            }
        }

        Ok((self, session))
    }
}
