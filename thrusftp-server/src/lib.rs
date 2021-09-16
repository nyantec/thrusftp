#[cfg(feature = "thrussh-server")]
pub mod thrussh;

use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;

use thrusftp_protocol::{Fs, FsHandle};
use thrusftp_protocol::types::*;
use thrusftp_protocol::parse::Serialize;

struct SftpClient<T: Fs + Send + Sync> {
    handles: HashMap<String, FsHandle<T::FileHandle, T::DirHandle>>,
}

pub struct SftpServer<T: Fs + Send + Sync> {
    clients: RwLock<HashMap<String, Arc<RwLock<SftpClient<T>>>>>,
    fs: T,
}

impl<T: Fs + Send + Sync> SftpServer<T> {
    pub fn new(fs: T) -> Arc<Self> {
        Arc::new(Self { fs, clients: RwLock::new(HashMap::new()) })
    }
    pub async fn create_client_handle(self: Arc<Self>, start_str: &str) -> String {
        let mut clients = self.clients.write().await;
        let mut num = 0u64;
        let mut handle;
        loop {
            handle = format!("{}{}", start_str, num);
            if !clients.contains_key(&handle) { break; }
            num += 1;
        }
        clients.insert(handle.clone(), Arc::new(RwLock::new(SftpClient { handles: Default::default() })));
        handle
    }

    pub async fn process(self: Arc<Self>, client_handle: &str, packet: SftpClientPacket) -> SftpServerPacket {
        let client = {
            let clients = self.clients.read().await;
            let client = clients.get(client_handle).unwrap().clone();
            client
        };
        self.process_internal(client, packet).await
    }

    async fn process_internal(self: Arc<Self>, client: Arc<RwLock<SftpClient<T>>>, packet: SftpClientPacket) -> SftpServerPacket {
        let mut client = client.write().await;
        match packet {
            SftpClientPacket::Init { .. } => {
                let mut extensions = vec![];
                if self.fs.statvfs_supported().await {
                    extensions.push(Extension {
                        name: "statvfs@openssh.com".to_string(),
                        data: "2".to_string(),
                    });
                }
                if self.fs.posix_rename_supported().await {
                    extensions.push(Extension {
                        name: "posix-rename@openssh.com".to_string(),
                        data: "1".to_string(),
                    });
                }
                if self.fs.fsync_supported().await {
                    extensions.push(Extension {
                        name: "fsync@openssh.com".to_string(),
                        data: "1".to_string(),
                    });
                }
                if self.fs.hardlink_supported().await {
                    extensions.push(Extension {
                        name: "hardlink@openssh.com".to_string(),
                        data: "1".to_string(),
                    });
                }
                SftpServerPacket::Version {
                    version: 3,
                    extensions: extensions.into(),
                }
            },
            SftpClientPacket::Realpath { id, path } => {
                self.fs.realpath(path).await
                    .map(|filename| {
                        SftpServerPacket::Name {
                            id,
                            names: vec![
                                Name {
                                    filename,
                                    ..Default::default()
                                },
                            ],
                        }
                    })
                    .unwrap_or_else(|err| error_resp(id, err))
            },
            SftpClientPacket::Opendir { id, path } => {
                let mut num = 0u64;
                let mut handle;
                loop {
                    handle = format!("{}{}", path, num);
                    if !client.handles.contains_key(&handle) { break; }
                    num += 1;
                }

                self.fs.opendir(path).await
                    .map(|dir| {
                        client.handles.insert(handle.clone(), FsHandle::Dir(dir));
                        SftpServerPacket::Handle { id, handle }
                    })
                    .unwrap_or_else(|err| error_resp(id, err))
            },
            SftpClientPacket::Readdir { id, handle } => {
                match client.handles.get_mut(&handle) {
                    Some(FsHandle::Dir(dir)) => {
                        self.fs.readdir(dir).await
                            .map(|names| SftpServerPacket::Name { id, names })
                            .unwrap_or_else(|err| error_resp(id, err))
                    },
                    _ => status_resp(id, StatusCode::BadMessage),
                }
            },
            SftpClientPacket::Close { id, handle } => {
                match client.handles.remove(&handle) {
                    Some(fs_handle) => {
                        result_resp(id, self.fs.close(fs_handle).await)
                    },
                    _ => status_resp(id, StatusCode::BadMessage),
                }
            },
            SftpClientPacket::Lstat { id, path } => {
                self.fs.lstat(path).await
                    .map(|attrs| SftpServerPacket::Attrs { id, attrs: attrs.into() })
                    .unwrap_or_else(|err| error_resp(id, err))
            },
            SftpClientPacket::Stat { id, path } => {
                self.fs.stat(path).await
                    .map(|attrs| SftpServerPacket::Attrs { id, attrs: attrs.into() })
                    .unwrap_or_else(|err| error_resp(id, err))
            },
            SftpClientPacket::Fstat { id, handle } => {
                match client.handles.get_mut(&handle) {
                    Some(FsHandle::File(file)) => {
                        self.fs.fstat(file).await
                            .map(|attrs| SftpServerPacket::Attrs { id, attrs: attrs.into() })
                            .unwrap_or_else(|err| error_resp(id, err))
                    },
                    _ => status_resp(id, StatusCode::BadMessage),
                }
            },
            SftpClientPacket::Open { id, filename, pflags, attrs } => {
                let mut num = 0u64;
                let mut handle;
                loop {
                    handle = format!("{}{}", filename, num);
                    if !client.handles.contains_key(&handle) { break; }
                    num += 1;
                }

                self.fs.open(filename, pflags, attrs).await
                    .map(|file| {
                        client.handles.insert(handle.clone(), FsHandle::File(file));
                        SftpServerPacket::Handle { id, handle }
                    })
                    .unwrap_or_else(|err| error_resp(id, err))
            },
            SftpClientPacket::Read { id, handle, offset, len } => {
                match client.handles.get_mut(&handle) {
                    Some(FsHandle::File(file)) => {
                        self.fs.read(file, offset, len).await
                            .map(|data| SftpServerPacket::Data { id, data: data.into() })
                            .unwrap_or_else(|err| error_resp(id, err))
                    },
                    _ => status_resp(id, StatusCode::BadMessage),
                }
            },
            SftpClientPacket::Write { id, handle, offset, data } => {
                match client.handles.get_mut(&handle) {
                    Some(FsHandle::File(file)) => {
                        result_resp(id, self.fs.write(file, offset, data.0).await)
                    },
                    _ => status_resp(id, StatusCode::BadMessage),
                }
            },
            SftpClientPacket::Setstat { id, path, attrs } => {
                result_resp(id, self.fs.setstat(path, attrs).await)
            },
            SftpClientPacket::Fsetstat { id, handle, attrs } => {
                match client.handles.get_mut(&handle) {
                    Some(FsHandle::File(file)) => {
                        result_resp(id, self.fs.fsetstat(file, attrs).await)
                    },
                    _ => status_resp(id, StatusCode::BadMessage),
                }
            },
            SftpClientPacket::Remove { id, filename } => {
                result_resp(id, self.fs.remove(filename).await)
            },
            SftpClientPacket::Mkdir { id, path, attrs } => {
                result_resp(id, self.fs.mkdir(path, attrs).await)
            },
            SftpClientPacket::Rmdir { id, path } => {
                result_resp(id, self.fs.rmdir(path).await)
            },
            SftpClientPacket::Rename { id, oldpath, newpath } => {
                result_resp(id, self.fs.rename(oldpath, newpath).await)
            },
            SftpClientPacket::Symlink { id, linkpath, targetpath } => {
                result_resp(id, self.fs.symlink(linkpath, targetpath).await)
            },
            SftpClientPacket::Readlink { id, path } => {
                self.fs.readlink(path).await
                    .map(|filename| {
                        SftpServerPacket::Name {
                            id,
                            names: vec![
                                Name {
                                    filename,
                                    ..Default::default()
                                },
                            ],
                        }
                    })
                    .unwrap_or_else(|err| error_resp(id, err))
            },
            SftpClientPacket::Extended { id, extended_request } => {
                match extended_request {
                    ExtendedRequest::OpensshStatvfs { path } => {
                        self.fs.statvfs(path).await
                            .map(|stats| {
                                let mut data = vec![];
                                stats.serialize(&mut data).unwrap();
                                SftpServerPacket::ExtendedReply {
                                    id,
                                    data: data.into(),
                                }
                            })
                            .unwrap_or_else(|err| error_resp(id, err))
                    },
                    ExtendedRequest::OpensshPosixRename { oldpath, newpath } => {
                        result_resp(id, self.fs.posix_rename(oldpath, newpath).await)
                    },
                    ExtendedRequest::OpensshHardlink { oldpath, newpath } => {
                        result_resp(id, self.fs.hardlink(oldpath, newpath).await)
                    },
                    ExtendedRequest::OpensshFsync { handle } => {
                        match client.handles.get_mut(&handle) {
                            Some(FsHandle::File(file)) => {
                                result_resp(id, self.fs.fsync(file).await)
                            },
                            _ => status_resp(id, StatusCode::BadMessage),
                        }
                    },
                }
            },
        }
    }
}

fn status_resp(id: u32, status_code: StatusCode) -> SftpServerPacket {
    SftpServerPacket::Status {
        id, status_code,
        error_message: format!("{:?}", status_code),
        language_tag: "en".to_string(),
    }
}

fn error_resp(id: u32, err: anyhow::Error) -> SftpServerPacket{
    let mut status_code = StatusCode::Failure;
    if let Some(ref io_err) = err.downcast_ref::<std::io::Error>() {
        status_code = match io_err.kind() {
            std::io::ErrorKind::NotFound => StatusCode::NoSuchFile,
            std::io::ErrorKind::UnexpectedEof => StatusCode::Eof,
            std::io::ErrorKind::PermissionDenied => StatusCode::PermissionDenied,
            std::io::ErrorKind::Unsupported => StatusCode::OpUnsupported,
            std::io::ErrorKind::InvalidInput => StatusCode::BadMessage,
            std::io::ErrorKind::InvalidData => StatusCode::BadMessage,
            _ => StatusCode::Failure,
        };
    };
    SftpServerPacket::Status {
        id, status_code,
        error_message: err.to_string(),
        language_tag: "en".to_string(),
    }
}

fn result_resp<T>(id: u32, r: anyhow::Result<T>) -> SftpServerPacket {
    match r {
        Err(e) => error_resp(id, e),
        Ok(_) => status_resp(id, StatusCode::r#Ok),
    }
}
