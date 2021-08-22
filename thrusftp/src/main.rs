mod protocol;
mod fs_sync;
mod fs_async;

use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt, SeekFrom};
use thrussh::*;
use thrussh::server::Session;
use async_trait::async_trait;
use std::convert::TryInto;
use std::sync::Arc;
use std::collections::HashMap;
use std::fs::{Metadata, Permissions};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use protocol::*;

#[tokio::main]
async fn main() {
    let mut config = thrussh::server::Config::default();
    config.connection_timeout = Some(std::time::Duration::from_secs(300));
    config.auth_rejection_time = std::time::Duration::from_millis(300);
    config.keys.push(thrussh_keys::key::KeyPair::generate_ed25519().unwrap());
    let server = Server{};
    thrussh::server::run(Arc::new(config), "0.0.0.0:2222", server).await.unwrap();
}

#[derive(Clone, Debug)]
struct Server { }

impl server::Server for Server {
    type Handler = Client;
    fn new(&mut self, _: Option<std::net::SocketAddr>) -> Client {
        Client::default()
    }
}

impl From<Metadata> for Attrs {
    fn from(metadata: Metadata) -> Attrs {
        Attrs {
            size: Some(metadata.len()),
            uid_gid: Some((metadata.uid(), metadata.gid())),
            permissions: Some(metadata.permissions().mode()),
            atime_mtime: Some((metadata.atime() as u32, metadata.mtime() as u32)),
            extended_attrs: vec![],
        }
    }
}

async fn apply_attrs_path(path: String, attrs: Attrs) -> std::io::Result<()> {
    if let Some(permissions) = attrs.permissions {
        fs::set_permissions(&path, Permissions::from_mode(permissions)).await?;
    }
    if let Some(size) = attrs.size {
        fs_async::truncate64(&path, size).await?;
    }
    Ok(())
}

async fn apply_attrs_file(file: &mut fs::File, attrs: Attrs) -> std::io::Result<()> {
    if let Some(permissions) = attrs.permissions {
        file.set_permissions(Permissions::from_mode(permissions)).await?;
    }
    if let Some(size) = attrs.size {
        file.set_len(size).await?;
    }
    Ok(())
}

#[derive(Default, Debug)]
struct Client {
    file_handles: HashMap<Handle, fs::File>,
    dir_handles: HashMap<Handle, fs::ReadDir>,
    recv_buf: Vec<u8>,
}

#[async_trait]
impl server::Handler for Client {
    type Error = anyhow::Error;

    async fn shell_request(self, channel: ChannelId, mut session: Session) -> anyhow::Result<(Self, Session)> {
        session.channel_success(channel);
        session.data(channel, CryptoVec::from_slice(b"Only SFTP allowed, bye\n"));
        session.flush()?;
        session.close(channel);
        Ok((self, session))
    }

    async fn subsystem_request(self, channel: ChannelId, name: &str, mut session: Session) -> anyhow::Result<(Self, Session)> {
        match name {
            "sftp" => session.channel_success(channel),
            _ => {
                session.channel_failure(channel);
                session.close(channel);
            },
        };
        Ok((self, session))
    }

    async fn auth_publickey(self, _: &str, _: &thrussh_keys::key::PublicKey) -> anyhow::Result<(Self, server::Auth)> {
        Ok((self, server::Auth::Accept))
    }

    async fn data(mut self, channel: ChannelId, mut data: &[u8], mut session: Session) -> anyhow::Result<(Self, Session)> {
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
                    let packet = SftpClientPacket::from_bytes(&self.recv_buf).unwrap();
                    self.recv_buf.clear();

                    let resp = self.process_packet(packet).await.unwrap();

                    let resp_bytes = resp.to_bytes().unwrap();
                    session.data(channel, CryptoVec::from_slice(&resp_bytes));
                }
            }
        }

        Ok((self, session))
    }
}

impl Client {
    async fn process_packet(&mut self, packet: SftpClientPacket) -> anyhow::Result<SftpServerPacket> {
        //println!("request: {:?}", packet);
        let resp = match packet {
            SftpClientPacket::Init { .. } => {
                SftpServerPacket::Version {
                    version: 3,
                    extensions: vec![
                        Extension {
                            name: "statvfs@openssh.com".to_string(),
                            data: "2".to_string(),
                        },
                        Extension {
                            name: "posix-rename@openssh.com".to_string(),
                            data: "1".to_string(),
                        },
                        Extension {
                            name: "fsync@openssh.com".to_string(),
                            data: "1".to_string(),
                        },
                        Extension {
                            name: "hardlink@openssh.com".to_string(),
                            data: "1".to_string(),
                        },
                    ],
                }
            },
            SftpClientPacket::Realpath { id, path } => {
                let canonicalized = fs::canonicalize(path).await?;
                SftpServerPacket::Name {
                    id,
                    names: vec![
                        Name {
                            filename: canonicalized.to_string_lossy().to_string(),
                            longname: "".to_string(),
                            attrs: Attrs {
                                size: None,
                                uid_gid: None,
                                permissions: None,
                                atime_mtime: None,
                                extended_attrs: vec![],
                            },
                        },
                    ],
                }
            },
            SftpClientPacket::Opendir { id, path } => {
                let mut num = 0u64;
                let mut handle;
                loop {
                    handle = format!("{}{}", path, num);
                    if !self.dir_handles.contains_key(&handle) { break; }
                    num += 1;
                }

                self.dir_handles.insert(handle.clone(), fs::read_dir(path).await?);
                SftpServerPacket::Handle {
                    id,
                    handle,
                }
            },
            SftpClientPacket::Readdir { id, handle } => {
                let iter = self.dir_handles.get_mut(&handle).ok_or(ProtocolError::NoSuchHandle)?;

                if let Some(e) = iter.next_entry().await? {
                    let metadata = e.metadata().await?;
                    SftpServerPacket::Name {
                        id,
                        names: vec![
                            Name {
                                filename: e.file_name().to_string_lossy().to_string(),
                                longname: e.file_name().to_string_lossy().to_string(),
                                attrs: metadata.into(),
                            }
                        ]
                    }
                } else {
                    status_resp(id, StatusCode::Eof)
                }
            },
            SftpClientPacket::Close { id, handle } => {
                if let Some(read_dir) = self.dir_handles.remove(&handle) {
                    drop(read_dir);
                    status_resp(id, StatusCode::r#Ok)
                } else if let Some(mut file) = self.file_handles.remove(&handle) {
                    file.flush().await?;
                    drop(file);
                    status_resp(id, StatusCode::r#Ok)
                } else {
                    Err(ProtocolError::NoSuchHandle)?
                }
            },
            SftpClientPacket::Lstat { id, path } => {
                match fs::symlink_metadata(path).await {
                    Err(e) => io_error_resp(id, e),
                    Ok(attrs) => SftpServerPacket::Attrs { id, attrs: attrs.into() },
                }
            },
            SftpClientPacket::Stat { id, path } => {
                match fs::metadata(path).await {
                    Err(e) => io_error_resp(id, e),
                    Ok(attrs) => SftpServerPacket::Attrs { id, attrs: attrs.into() },
                }
            },
            SftpClientPacket::Fstat { id, handle } => {
                let file = self.file_handles.get(&handle).ok_or(ProtocolError::NoSuchHandle)?;
                match file.metadata().await {
                    Err(e) => io_error_resp(id, e),
                    Ok(attrs) => SftpServerPacket::Attrs { id, attrs: attrs.into() },
                }
            },
            SftpClientPacket::Open { id, filename, pflags, attrs } => {
                let mut num = 0u64;
                let mut handle;
                loop {
                    handle = format!("{}{}", filename, num);
                    if !self.file_handles.contains_key(&handle) { break; }
                    num += 1;
                }

                let mut options = fs::OpenOptions::new();
                if pflags.read   { options.read(true); }
                if pflags.write  { options.write(true); }
                if pflags.append { options.append(true); }
                if pflags.creat  { options.create(true); }
                if pflags.trunc  { options.truncate(true); }
                if pflags.excl   { options.create_new(true); }
                if let Some(permissions) = attrs.permissions {
                    options.mode(permissions);
                }
                match options.open(filename).await {
                    Err(e) => io_error_resp(id, e),
                    Ok(file) => {
                        self.file_handles.insert(handle.clone(), file);
                        SftpServerPacket::Handle { id, handle }
                    },
                }
            },
            SftpClientPacket::Read { id, handle, offset, len } => {
                let file = self.file_handles.get_mut(&handle).ok_or(ProtocolError::NoSuchHandle)?;

                file.seek(SeekFrom::Start(offset)).await?;
                let mut data = vec![0u8; len as usize];
                let mut read_len = 1;
                let mut total_read_len = 0;
                while read_len != 0 && total_read_len < len as usize {
                    read_len = file.read(&mut data[read_len..]).await?;
                    total_read_len += read_len;
                }
                if total_read_len == 0 {
                    status_resp(id, StatusCode::Eof)
                } else {
                    data.truncate(total_read_len);
                    SftpServerPacket::Data { id, data }
                }
            },
            SftpClientPacket::Write { id, handle, offset, data } => {
                let file = self.file_handles.get_mut(&handle).ok_or(ProtocolError::NoSuchHandle)?;

                file.seek(SeekFrom::Start(offset)).await?;
                file.write_all(&data).await?;
                status_resp(id, StatusCode::r#Ok)
            },
            SftpClientPacket::Setstat { id, path, attrs } => {
                io_result_resp(id, apply_attrs_path(path, attrs).await)
            },
            SftpClientPacket::Fsetstat { id, handle, attrs } => {
                let file = self.file_handles.get_mut(&handle).ok_or(ProtocolError::NoSuchHandle)?;
                io_result_resp(id, apply_attrs_file(file, attrs).await)
            },
            SftpClientPacket::Remove { id, filename } => {
                io_result_resp(id, fs::remove_file(filename).await)
            },
            SftpClientPacket::Mkdir { id, path, attrs } => {
                // TODO attrs
                // https://github.com/rust-lang/rust/issues/22415
                io_result_resp(id, fs::create_dir(path).await)
            },
            SftpClientPacket::Rmdir { id, path } => {
                io_result_resp(id, fs::remove_dir(path).await)
            },
            SftpClientPacket::Rename { id, oldpath, newpath } => {
                if fs::metadata(&newpath).await.is_ok() {
                    // target already exists
                    io_error_resp(id, std::io::ErrorKind::AlreadyExists.into())
                } else {
                    io_result_resp(id, fs::rename(oldpath, newpath).await)
                }
            },
            SftpClientPacket::Symlink { id, linkpath, targetpath } => {
                io_result_resp(id, fs::symlink(targetpath, linkpath).await)
            },
            SftpClientPacket::Readlink { id, path } => {
                match fs::read_link(path).await {
                    Err(e) => io_error_resp(id, e),
                    Ok(target) => SftpServerPacket::Name {
                        id,
                        names: vec![
                            Name {
                                filename: target.to_string_lossy().to_string(),
                                longname: "".to_string(),
                                attrs: Attrs {
                                    size: None,
                                    uid_gid: None,
                                    permissions: None,
                                    atime_mtime: None,
                                    extended_attrs: vec![],
                                },
                            },
                        ],
                    },
                }
            },
            SftpClientPacket::Extended { id, extended_request, data } => {
                match extended_request.as_str() {
                    "statvfs@openssh.com" => {
                        let mut data = data.as_slice();
                        let path = read_string!(data);
                        if data.len() != 0 { Err(ProtocolError::InvalidLength)? }
                        match fs_async::statvfs(path).await {
                            Err(e) => io_error_resp(id, e),
                            Ok(stat) => {
                                let data = statvfs_to_bytes(stat);
                                SftpServerPacket::ExtendedReply { id, data }
                            },
                        }
                    },
                    "posix-rename@openssh.com" => {
                        let mut data = data.as_slice();
                        let oldpath = read_string!(data);
                        let newpath = read_string!(data);
                        if data.len() != 0 { Err(ProtocolError::InvalidLength)? }
                        io_result_resp(id, fs::rename(oldpath, newpath).await)
                    },
                    "hardlink@openssh.com" => {
                        let mut data = data.as_slice();
                        let oldpath = read_string!(data);
                        let newpath = read_string!(data);
                        if data.len() != 0 { Err(ProtocolError::InvalidLength)? }
                        io_result_resp(id, fs::hard_link(oldpath, newpath).await)
                    },
                    "fsync@openssh.com" => {
                        let mut data = data.as_slice();
                        let handle = read_string!(data);
                        if data.len() != 0 { Err(ProtocolError::InvalidLength)? }
                        let file = self.file_handles.get_mut(&handle).ok_or(ProtocolError::NoSuchHandle)?;
                        io_result_resp(id, file.sync_all().await)
                    },
                    _ => status_resp(id, StatusCode::OpUnsupported),
                }
            },
        };
        //println!("response: {:?}", resp);
        Ok(resp)
    }
}

fn status_resp(id: u32, status_code: StatusCode) -> SftpServerPacket {
    SftpServerPacket::Status {
        id, status_code,
        error_message: format!("{:?}", status_code),
        language_tag: "en".to_string(),
    }
}

fn io_error_resp(id: u32, e: std::io::Error) -> SftpServerPacket {
    let status_code = match e.kind() {
        std::io::ErrorKind::NotFound => StatusCode::NoSuchFile,
        std::io::ErrorKind::PermissionDenied => StatusCode::PermissionDenied,
        std::io::ErrorKind::InvalidInput => StatusCode::BadMessage,
        std::io::ErrorKind::InvalidData => StatusCode::BadMessage,
        _ => StatusCode::Failure,
    };
    SftpServerPacket::Status {
        id, status_code,
        error_message: e.to_string(),
        language_tag: "en".to_string(),
    }
}

fn io_result_resp<T>(id: u32, r: std::io::Result<T>) -> SftpServerPacket {
    match r {
        Err(e) => io_error_resp(id, e),
        Ok(_) => status_resp(id, StatusCode::r#Ok),
    }
}
