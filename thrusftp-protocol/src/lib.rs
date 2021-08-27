use async_trait::async_trait;
use anyhow::Result;
use crate::types::{Attrs, Pflags, Name, FsStats};

pub mod parse;
pub mod types;

pub enum FsHandle<F, D> {
    File(F),
    Dir(D),
}

#[async_trait]
pub trait Fs {
    type FileHandle: Send + Sync;
    type DirHandle: Send + Sync;

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

    async fn posix_rename_supported(&self) -> bool { false }
    async fn posix_rename(&self, _oldpath: String, _newpath: String) -> Result<()> {
        Err(std::io::Error::from(std::io::ErrorKind::Unsupported).into())
    }
    async fn fsync_supported(&self) -> bool { false }
    async fn fsync(&self, _handle: &mut Self::FileHandle) -> Result<()> {
        Err(std::io::Error::from(std::io::ErrorKind::Unsupported).into())
    }
    async fn statvfs_supported(&self) -> bool { false }
    async fn statvfs(&self, _path: String) -> Result<FsStats> {
        Err(std::io::Error::from(std::io::ErrorKind::Unsupported).into())
    }
    async fn hardlink_supported(&self) -> bool { false }
    async fn hardlink(&self, _oldpath: String, _newpath: String) -> Result<()> {
        Err(std::io::Error::from(std::io::ErrorKind::Unsupported).into())
    }
}

