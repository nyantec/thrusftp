use tokio::sync::RwLock;
use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;

use crate::types::*;
use crate::error::Result;
use crate::parse::Serialize;

pub enum FsHandle<F, D> {
    File(F),
    Dir(D),
}

struct SftpClient<T: Fs> {
    handles: HashMap<String, FsHandle<T::FileHandle, T::DirHandle>>,
}

pub struct SftpServer<T: Fs> {
    clients: RwLock<HashMap<String, Arc<RwLock<SftpClient<T>>>>>,
    fs: T,
}

#[async_trait]
pub trait Fs {
    type FileHandle;
    type DirHandle;

    async fn open(&self, filename: String, pflags: Pflags, attrs: Attrs) -> Result<Self::FileHandle>;
    async fn close(&self, handle: FsHandle<Self::FileHandle, Self::DirHandle>) -> Result<()>;
    async fn read(&self, handle: &mut Self::FileHandle, offset: u64, len: u32) -> Result<Vec<u8>>;
    async fn write(&self, handle: &mut Self::FileHandle, offset: u64, data: Vec<u8>) -> Result<()>;
    async fn lstat(&self, path: String) -> Result<Attrs>;
    async fn fstat(&self, handle: &mut Self::FileHandle) -> Result<Attrs>;
    async fn setstat(&self, path: String, attrs: Attrs) -> Result<()>;
    async fn fsetstat(&self, handle: &mut Self::FileHandle, attrs: Attrs) -> Result<()>;
    async fn opendir(&self, path: String) -> Result<Self::DirHandle>;
    async fn readdir(&self, handle: &mut Self::DirHandle) -> Result<Vec<Name>>;
    async fn remove(&self, filename: String) -> Result<()>;
    async fn mkdir(&self, path: String, attrs: Attrs) -> Result<()>;
    async fn rmdir(&self, path: String) -> Result<()>;
    async fn realpath(&self, path: String) -> Result<String>;
    async fn stat(&self, path: String) -> Result<Attrs>;
    async fn rename(&self, oldpath: String, newpath: String) -> Result<()>;
    async fn readlink(&self, path: String) -> Result<String>;
    async fn symlink(&self, linkpath: String, targetpath: String) -> Result<()>;

    async fn posix_rename(&self, oldpath: String, newpath: String) -> Result<()>;
    async fn fsync(&self, handle: &mut Self::FileHandle) -> Result<()>;
    async fn statvfs(&self, path: String) -> Result<FsStats>;
    async fn hardlink(&self, oldpath: String, newpath: String) -> Result<()>;
}

impl<T: Fs> SftpServer<T> {
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
                    ].into(),
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
                                SftpServerPacket::ExtendedReply {
                                    id,
                                    data: stats.serialize().unwrap().into()
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
