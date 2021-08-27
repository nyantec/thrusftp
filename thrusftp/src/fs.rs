use std::fs::{Metadata, Permissions};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use tokio::fs;
use tokio::io::{AsyncSeekExt, AsyncReadExt, AsyncWriteExt, SeekFrom};
use async_trait::async_trait;

use crate::fs_async;
use crate::server::{Fs, FsHandle};
use crate::error::Result;
use crate::types::{Attrs, Pflags, Name, FsStats};

pub struct LocalFs;

async fn apply_attrs_path(path: String, attrs: Attrs) -> std::io::Result<()> {
    if let Some(permissions) = attrs.permissions {
        fs::set_permissions(&path, Permissions::from_mode(permissions)).await?;
    }
    if let Some(size) = attrs.size {
        fs_async::truncate64(&path, size).await?;
    }
    Ok(())
}

async fn apply_attrs_handle(handle: &mut fs::File, attrs: Attrs) -> std::io::Result<()> {
    if let Some(permissions) = attrs.permissions {
        handle.set_permissions(Permissions::from_mode(permissions)).await?;
    }
    if let Some(size) = attrs.size {
        handle.set_len(size).await?;
    }
    Ok(())
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

impl From<libc::statvfs> for FsStats {
    fn from(f: libc::statvfs) -> Self {
        Self {
            f_bsize: f.f_bsize,
            f_frsize: f.f_frsize,
            f_blocks: f.f_blocks,
            f_bfree: f.f_bfree,
            f_bavail: f.f_bavail,
            f_files: f.f_files,
            f_ffree: f.f_ffree,
            f_favail: f.f_favail,
            f_fsid: f.f_fsid,
            f_flag: f.f_flag,
            f_namemax: f.f_namemax,
        }
    }
}

#[async_trait]
impl Fs for LocalFs {
    type FileHandle = tokio::fs::File;
    type DirHandle = tokio::fs::ReadDir;

    async fn open(&self, filename: String, pflags: Pflags, attrs: Attrs) -> Result<Self::FileHandle> {
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
        Ok(options.open(filename).await?)
    }
    async fn close(&self, handle: FsHandle<Self::FileHandle, Self::DirHandle>) -> Result<()> {
        match handle {
            FsHandle::File(mut file) => {
                file.flush().await?;
                drop(file);
            },
            FsHandle::Dir(dir) => {
                drop(dir);
            },
        }
        Ok(())
    }
    async fn read(&self, handle: &mut Self::FileHandle, offset: u64, len: u32) -> Result<Vec<u8>> {
        handle.seek(SeekFrom::Start(offset)).await?;
        let mut data = vec![0u8; len as usize];
        let mut read_len = 0;
        let mut total_read_len = 0;
        loop {
            if total_read_len >= len as usize { break; }
            read_len = handle.read(&mut data[read_len..]).await?;
            total_read_len += read_len;
            if read_len == 0 { break; }
        }
        if total_read_len == 0 {
            Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof).into())
        } else {
            data.truncate(total_read_len);
            Ok(data)
        }
    }
    async fn write(&self, handle: &mut Self::FileHandle, offset: u64, data: Vec<u8>) -> Result<()> {
        handle.seek(SeekFrom::Start(offset)).await?;
        handle.write_all(&data).await?;
        Ok(())
    }
    async fn lstat(&self, path: String) -> Result<Attrs> {
        Ok(fs::symlink_metadata(path).await?.into())
    }
    async fn fstat(&self, handle: &mut Self::FileHandle) -> Result<Attrs> {
        Ok(handle.metadata().await?.into())
    }
    async fn setstat(&self, path: String, attrs: Attrs) -> Result<()> {
        Ok(apply_attrs_path(path, attrs).await?)
    }
    async fn fsetstat(&self, handle: &mut Self::FileHandle, attrs: Attrs) -> Result<()> {
        Ok(apply_attrs_handle(handle, attrs).await?)
    }
    async fn opendir(&self, path: String) -> Result<Self::DirHandle> {
        Ok(fs::read_dir(path).await?)
    }
    async fn readdir(&self, handle: &mut Self::DirHandle) -> Result<Vec<Name>> {
        if let Some(e) = handle.next_entry().await? {
            let metadata = e.metadata().await?;
            Ok(vec![
                Name {
                    filename: e.file_name().to_string_lossy().to_string(),
                    longname: e.file_name().to_string_lossy().to_string(),
                    attrs: metadata.into(),
                }
            ])
        } else {
            Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof).into())
        }
    }
    async fn remove(&self, filename: String) -> Result<()> {
        Ok(fs::remove_file(filename).await?)
    }
    async fn mkdir(&self, path: String, attrs: Attrs) -> Result<()> {
        // TODO attrs
        // https://github.com/rust-lang/rust/issues/22415
        Ok(fs::create_dir(path).await?)
    }
    async fn rmdir(&self, path: String) -> Result<()> {
        Ok(fs::remove_dir(path).await?)
    }
    async fn realpath(&self, path: String) -> Result<String> {
        Ok(fs::canonicalize(path).await?.to_string_lossy().to_string())
    }
    async fn stat(&self, path: String) -> Result<Attrs> {
        Ok(fs::metadata(path).await?.into())
    }
    async fn rename(&self, oldpath: String, newpath: String) -> Result<()> {
        if fs::metadata(&newpath).await.is_ok() {
            Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists).into())
        } else {
            Ok(fs::rename(oldpath, newpath).await?)
        }
    }
    async fn readlink(&self, path: String) -> Result<String> {
        Ok(fs::read_link(path).await
            .map(|target| target.to_string_lossy().to_string())?)
    }
    async fn symlink(&self, linkpath: String, targetpath: String) -> Result<()> {
        Ok(fs::symlink(targetpath, linkpath).await?)
    }
    async fn posix_rename(&self, oldpath: String, newpath: String) -> Result<()> {
        Ok(fs::rename(oldpath, newpath).await?)
    }
    async fn fsync(&self, handle: &mut Self::FileHandle) -> Result<()> {
        Ok(handle.sync_all().await?)
    }
    async fn statvfs(&self, path: String) -> Result<FsStats> {
        Ok(fs_async::statvfs(path).await?.into())
    }
    async fn hardlink(&self, oldpath: String, newpath: String) -> Result<()> {
        Ok(fs::hard_link(oldpath, newpath).await?)
    }
}

