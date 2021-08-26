use crate::parse::{Serialize, Deserialize};
use bin_ser::{Serialize, Deserialize};

#[derive(Clone, Debug)]
pub struct Pflags {
    pub read: bool,
    pub write: bool,
    pub append: bool,
    pub creat: bool,
    pub trunc: bool,
    pub excl: bool,
}

#[derive(Clone, Debug)]
pub struct Attrsflags {
    pub size: bool,
    pub uidgid: bool,
    pub permissions: bool,
    pub acmodtime: bool,
    pub extended: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtendedAttr {
    pub r#type: String,
    pub data: String,
}

#[derive(Clone, Debug, Default)]
pub struct Attrs {
    pub size: Option<u64>,
    pub uid_gid: Option<(u32, u32)>,
    pub permissions: Option<u32>,
    pub atime_mtime: Option<(u32, u32)>,
    pub extended_attrs: Vec<ExtendedAttr>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[repr(u32)]
pub enum StatusCode {
    r#Ok = 0,
    Eof = 1,
    NoSuchFile = 2,
    PermissionDenied = 3,
    Failure = 4,
    BadMessage = 5,
    NoConnection = 6,
    ConnectionLost = 7,
    OpUnsupported = 8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Extension {
    pub name: String,
    pub data: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Name {
    pub filename: String,
    pub longname: String,
    pub attrs: Attrs,
}

pub type Handle = String;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum SftpClientPacket {
    Init {
        version: u32,
        //extensions: Vec<Extension>,
    } = 1,
    Open {
        id: u32,
        filename: String,
        pflags: Pflags,
        attrs: Attrs,
    } = 3,
    Close {
        id: u32,
        handle: Handle,
    } = 4,
    Read {
        id: u32,
        handle: Handle,
        offset: u64,
        len: u32,
    } = 5,
    Write {
        id: u32,
        handle: Handle,
        offset: u64,
        data: Data,
    } = 6,
    Lstat {
        id: u32,
        path: String,
    } = 7,
    Fstat {
        id: u32,
        handle: Handle,
    } = 8,
    Setstat {
        id: u32,
        path: String,
        attrs: Attrs,
    } = 9,
    Fsetstat {
        id: u32,
        handle: Handle,
        attrs: Attrs,
    } = 10,
    Opendir {
        id: u32,
        path: String,
    } = 11,
    Readdir {
        id: u32,
        handle: Handle,
    } = 12,
    Remove {
        id: u32,
        filename: String,
    } = 13,
    Mkdir {
        id: u32,
        path: String,
        attrs: Attrs,
    } = 14,
    Rmdir {
        id: u32,
        path: String,
    } = 15,
    Realpath {
        id: u32,
        path: String,
    } = 16,
    Stat {
        id: u32,
        path: String,
    } = 17,
    Rename {
        id: u32,
        oldpath: String,
        newpath: String,
    } = 18,
    Readlink {
        id: u32,
        path: String,
    } = 19,
    Symlink {
        id: u32,
        linkpath: String,
        targetpath: String,
    } = 20,

    Extended {
        id: u32,
        extended_request: String,
        data: Data,
    } = 200,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum SftpServerPacket {
    Version {
        version: u32,
        //extensions: Vec<Extension>,
    } = 2,
    Status {
        id: u32,
        status_code: StatusCode,
        error_message: String,
        language_tag: String,
    } = 101,
    Handle {
        id: u32,
        handle: Handle,
    } = 102,
    Data {
        id: u32,
        data: Data,
    } = 103,
    Name {
        id: u32,
        names: Vec<Name>,
    } = 104,
    Attrs {
        id: u32,
        attrs: Attrs,
    } = 105,

    ExtendedReply {
        id: u32,
        data: Data,
    } = 201,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FsStats {
    pub f_bsize: u64,
    pub f_frsize: u64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_favail: u64,
    pub f_fsid: u64,
    pub f_flag: u64,
    pub f_namemax: u64,
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

#[derive(Clone, Debug)]
pub struct Data(pub Vec<u8>);

impl From<Vec<u8>> for Data {
    fn from(vec: Vec<u8>) -> Self {
        Self(vec)
    }
}
