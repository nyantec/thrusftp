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
#[bin_ser(repr = u32)]
pub enum StatusCode {
    #[bin_ser(num = 0)]
    r#Ok,
    #[bin_ser(num = 1)]
    Eof,
    #[bin_ser(num = 2)]
    NoSuchFile,
    #[bin_ser(num = 3)]
    PermissionDenied,
    #[bin_ser(num = 4)]
    Failure,
    #[bin_ser(num = 5)]
    BadMessage,
    #[bin_ser(num = 6)]
    NoConnection,
    #[bin_ser(num = 7)]
    ConnectionLost,
    #[bin_ser(num = 8)]
    OpUnsupported,
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
#[bin_ser(repr = u8)]
pub enum SftpClientPacket {
    #[bin_ser(num = 1)]
    Init {
        version: u32,
        //extensions: Vec<Extension>,
    },
    #[bin_ser(num = 3)]
    Open {
        id: u32,
        filename: String,
        pflags: Pflags,
        attrs: Attrs,
    },
    #[bin_ser(num = 4)]
    Close {
        id: u32,
        handle: Handle,
    },
    #[bin_ser(num = 5)]
    Read {
        id: u32,
        handle: Handle,
        offset: u64,
        len: u32,
    },
    #[bin_ser(num = 6)]
    Write {
        id: u32,
        handle: Handle,
        offset: u64,
        data: Data,
    },
    #[bin_ser(num = 7)]
    Lstat {
        id: u32,
        path: String,
    },
    #[bin_ser(num = 8)]
    Fstat {
        id: u32,
        handle: Handle,
    },
    #[bin_ser(num = 9)]
    Setstat {
        id: u32,
        path: String,
        attrs: Attrs,
    },
    #[bin_ser(num = 10)]
    Fsetstat {
        id: u32,
        handle: Handle,
        attrs: Attrs,
    },
    #[bin_ser(num = 11)]
    Opendir {
        id: u32,
        path: String,
    },
    #[bin_ser(num = 12)]
    Readdir {
        id: u32,
        handle: Handle,
    },
    #[bin_ser(num = 13)]
    Remove {
        id: u32,
        filename: String,
    },
    #[bin_ser(num = 14)]
    Mkdir {
        id: u32,
        path: String,
        attrs: Attrs,
    },
    #[bin_ser(num = 15)]
    Rmdir {
        id: u32,
        path: String,
    },
    #[bin_ser(num = 16)]
    Realpath {
        id: u32,
        path: String,
    },
    #[bin_ser(num = 17)]
    Stat {
        id: u32,
        path: String,
    },
    #[bin_ser(num = 18)]
    Rename {
        id: u32,
        oldpath: String,
        newpath: String,
    },
    #[bin_ser(num = 19)]
    Readlink {
        id: u32,
        path: String,
    },
    #[bin_ser(num = 20)]
    Symlink {
        id: u32,
        linkpath: String,
        targetpath: String,
    },

    #[bin_ser(num = 200)]
    Extended {
        id: u32,
        extended_request: String,
        data: Data,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[bin_ser(repr = u8)]
pub enum SftpServerPacket {
    #[bin_ser(num = 2)]
    Version {
        version: u32,
        //extensions: Vec<Extension>,
    },
    #[bin_ser(num = 101)]
    Status {
        id: u32,
        status_code: StatusCode,
        error_message: String,
        language_tag: String,
    },
    #[bin_ser(num = 102)]
    Handle {
        id: u32,
        handle: Handle,
    },
    #[bin_ser(num = 103)]
    Data {
        id: u32,
        data: Data,
    },
    #[bin_ser(num = 104)]
    Name {
        id: u32,
        names: Vec<Name>,
    },
    #[bin_ser(num = 105)]
    Attrs {
        id: u32,
        attrs: Attrs,
    },

    #[bin_ser(num = 201)]
    ExtendedReply {
        id: u32,
        data: Data,
    },
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
