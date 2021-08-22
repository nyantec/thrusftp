use tokio::task::spawn_blocking;
use std::io::Result;
use std::path::PathBuf;
use crate::fs_sync;

pub(crate) async fn statvfs<P: Into<PathBuf>>(path: P) -> Result<libc::statvfs> {
    let path: PathBuf = path.into();
    spawn_blocking(move || {
        fs_sync::statvfs(path)
    }).await?
}

pub(crate) async fn truncate64<P: Into<PathBuf>>(path: P, size: u64) -> Result<()> {
    let path: PathBuf = path.into();
    spawn_blocking(move || {
        fs_sync::truncate64(path, size)
    }).await?
}
