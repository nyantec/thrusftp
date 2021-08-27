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
    #[bin_ser(val = 0)]
    r#Ok,
    #[bin_ser(val = 1)]
    Eof,
    #[bin_ser(val = 2)]
    NoSuchFile,
    #[bin_ser(val = 3)]
    PermissionDenied,
    #[bin_ser(val = 4)]
    Failure,
    #[bin_ser(val = 5)]
    BadMessage,
    #[bin_ser(val = 6)]
    NoConnection,
    #[bin_ser(val = 7)]
    ConnectionLost,
    #[bin_ser(val = 8)]
    OpUnsupported,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Extension {
    pub name: String,
    pub data: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Name {
    pub filename: String,
    pub longname: String,
    pub attrs: Attrs,
}

#[derive(Clone, Debug)]
pub enum ExtendedRequestType {
    OpensshStatvfs,
    OpensshPosixRename,
    OpensshHardlink,
    OpensshFsync,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[bin_ser(repr = ExtendedRequestType)]
pub enum ExtendedRequest {
    #[bin_ser(val = ExtendedRequestType::OpensshStatvfs)]
    OpensshStatvfs {
        path: String,
    },
    #[bin_ser(val = ExtendedRequestType::OpensshPosixRename)]
    OpensshPosixRename {
        oldpath: String,
        newpath: String,
    },
    #[bin_ser(val = ExtendedRequestType::OpensshHardlink)]
    OpensshHardlink {
        oldpath: String,
        newpath: String,
    },
    #[bin_ser(val = ExtendedRequestType::OpensshFsync)]
    OpensshFsync {
        handle: String,
    },
}

pub type Handle = String;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[bin_ser(repr = u8)]
pub enum SftpClientPacket {
    #[bin_ser(val = 1)]
    Init {
        version: u32,
        extensions: VecEos<Extension>,
    },
    #[bin_ser(val = 3)]
    Open {
        id: u32,
        filename: String,
        pflags: Pflags,
        attrs: Attrs,
    },
    #[bin_ser(val = 4)]
    Close {
        id: u32,
        handle: Handle,
    },
    #[bin_ser(val = 5)]
    Read {
        id: u32,
        handle: Handle,
        offset: u64,
        len: u32,
    },
    #[bin_ser(val = 6)]
    Write {
        id: u32,
        handle: Handle,
        offset: u64,
        data: VecU8,
    },
    #[bin_ser(val = 7)]
    Lstat {
        id: u32,
        path: String,
    },
    #[bin_ser(val = 8)]
    Fstat {
        id: u32,
        handle: Handle,
    },
    #[bin_ser(val = 9)]
    Setstat {
        id: u32,
        path: String,
        attrs: Attrs,
    },
    #[bin_ser(val = 10)]
    Fsetstat {
        id: u32,
        handle: Handle,
        attrs: Attrs,
    },
    #[bin_ser(val = 11)]
    Opendir {
        id: u32,
        path: String,
    },
    #[bin_ser(val = 12)]
    Readdir {
        id: u32,
        handle: Handle,
    },
    #[bin_ser(val = 13)]
    Remove {
        id: u32,
        filename: String,
    },
    #[bin_ser(val = 14)]
    Mkdir {
        id: u32,
        path: String,
        attrs: Attrs,
    },
    #[bin_ser(val = 15)]
    Rmdir {
        id: u32,
        path: String,
    },
    #[bin_ser(val = 16)]
    Realpath {
        id: u32,
        path: String,
    },
    #[bin_ser(val = 17)]
    Stat {
        id: u32,
        path: String,
    },
    #[bin_ser(val = 18)]
    Rename {
        id: u32,
        oldpath: String,
        newpath: String,
    },
    #[bin_ser(val = 19)]
    Readlink {
        id: u32,
        path: String,
    },
    #[bin_ser(val = 20)]
    Symlink {
        id: u32,
        linkpath: String,
        targetpath: String,
    },

    #[bin_ser(val = 200)]
    Extended {
        id: u32,
        extended_request: ExtendedRequest,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[bin_ser(repr = u8)]
pub enum SftpServerPacket {
    #[bin_ser(val = 2)]
    Version {
        version: u32,
        extensions: VecEos<Extension>,
    },
    #[bin_ser(val = 101)]
    Status {
        id: u32,
        status_code: StatusCode,
        error_message: String,
        language_tag: String,
    },
    #[bin_ser(val = 102)]
    Handle {
        id: u32,
        handle: Handle,
    },
    #[bin_ser(val = 103)]
    Data {
        id: u32,
        data: VecU8,
    },
    #[bin_ser(val = 104)]
    Name {
        id: u32,
        names: Vec<Name>,
    },
    #[bin_ser(val = 105)]
    Attrs {
        id: u32,
        attrs: Attrs,
    },

    #[bin_ser(val = 201)]
    ExtendedReply {
        id: u32,
        data: VecU8,
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

/// Vec that has no length on-wire. It ends when the stream ends.
#[derive(Clone, Debug)]
pub struct VecEos<T>(pub Vec<T>);

impl<T> From<Vec<T>> for VecEos<T> {
    fn from(vec: Vec<T>) -> Self {
        Self(vec)
    }
}

#[derive(Clone, Debug)]
pub struct VecU8(pub Vec<u8>);

impl From<Vec<u8>> for VecU8 {
    fn from(vec: Vec<u8>) -> Self {
        Self(vec)
    }
}
