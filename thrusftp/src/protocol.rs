use std::io::Write;
use std::fmt::{Display, Formatter};
use std::convert::TryInto;

#[derive(Copy, Clone, Debug)]
pub enum ProtocolError {
    UnknownCommand,
    InvalidUtf8,
    InvalidLength, // Not all packet contents were parsed
    IncompleteBuffer, // length field > buffer size
    NoSuchHandle,
}

impl From<std::string::FromUtf8Error> for ProtocolError {
    fn from(_: std::string::FromUtf8Error) -> Self {
        ProtocolError::InvalidUtf8
    }
}

impl Display for ProtocolError {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        write!(fmt, "{:?}", self)
    }
}

impl std::error::Error for ProtocolError {}

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

#[derive(Clone, Debug)]
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

pub type Handle = String;

#[derive(Clone, Debug)]
pub enum SftpClientPacket {
    Init {
        version: u32,
        extensions: Vec<Extension>,
    },
    Open {
        id: u32,
        filename: String,
        pflags: Pflags,
        attrs: Attrs,
    },
    Close {
        id: u32,
        handle: Handle,
    },
    Read {
        id: u32,
        handle: Handle,
        offset: u64,
        len: u32,
    },
    Write {
        id: u32,
        handle: Handle,
        offset: u64,
        data: Vec<u8>,
    },
    Lstat {
        id: u32,
        path: String,
    },
    Fstat {
        id: u32,
        handle: Handle,
    },
    Setstat {
        id: u32,
        path: String,
        attrs: Attrs,
    },
    Fsetstat {
        id: u32,
        handle: Handle,
        attrs: Attrs,
    },
    Opendir {
        id: u32,
        path: String,
    },
    Readdir {
        id: u32,
        handle: Handle,
    },
    Remove {
        id: u32,
        filename: String,
    },
    Mkdir {
        id: u32,
        path: String,
        attrs: Attrs,
    },
    Rmdir {
        id: u32,
        path: String,
    },
    Realpath {
        id: u32,
        path: String,
    },
    Stat {
        id: u32,
        path: String,
    },
    Rename {
        id: u32,
        oldpath: String,
        newpath: String,
    },
    Readlink {
        id: u32,
        path: String,
    },
    Symlink {
        id: u32,
        linkpath: String,
        targetpath: String,
    },

    Extended {
        id: u32,
        extended_request: String,
        data: Vec<u8>,
    },
}

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub enum StatusCode {
    r#Ok,
    Eof,
    NoSuchFile,
    PermissionDenied,
    Failure,
    BadMessage,
    NoConnection,
    ConnectionLost,
    OpUnsupported,
}

#[derive(Clone, Debug)]
pub struct Extension {
    pub name: String,
    pub data: String,
}

#[derive(Clone, Debug)]
pub struct Name {
    pub filename: String,
    pub longname: String,
    pub attrs: Attrs,
}

#[derive(Clone, Debug)]
pub enum SftpServerPacket {
    Version {
        version: u32,
        extensions: Vec<Extension>,
    },
    Status {
        id: u32,
        status_code: StatusCode,
        error_message: String,
        language_tag: String,
    },
    Handle {
        id: u32,
        handle: Handle,
    },
    Data {
        id: u32,
        data: Vec<u8>,
    },
    Name {
        id: u32,
        names: Vec<Name>,
    },
    Attrs {
        id: u32,
        attrs: Attrs,
    },

    ExtendedReply {
        id: u32,
        data: Vec<u8>,
    },
}

#[macro_export]
macro_rules! write_uint {
    ($buf:expr, $num:expr) => {
        $buf.write_all(&$num.to_be_bytes()).unwrap();
    };
}
#[macro_export]
macro_rules! write_u32 {
    ($buf:expr, $num:expr) => {
        {
            let num_u32: u32 = $num.into();
            write_uint!($buf, num_u32);
        }
    };
}
#[macro_export]
macro_rules! write_u64 {
    ($buf:expr, $num:expr) => {
        {
            let num_u64: u64 = $num.into();
            write_uint!($buf, num_u64);
        }
    };
}
#[macro_export]
macro_rules! read_u64 {
    ($buf:expr) => {
        {
            let res = u64::from_be_bytes($buf[..8].try_into().unwrap());
            $buf = &$buf[8..];
            res
        }
    };
}
#[macro_export]
macro_rules! read_u32 {
    ($buf:expr) => {
        {
            let res = u32::from_be_bytes($buf[..4].try_into().unwrap());
            $buf = &$buf[4..];
            res
        }
    };
}
#[macro_export]
macro_rules! write_u8 {
    ($buf:expr, $num:expr) => {
        {
            $buf.write_all(&[$num]).unwrap();
        }
    };
}
#[macro_export]
macro_rules! read_u8 {
    ($buf:expr) => {
        {
            let res = $buf[0];
            $buf = &$buf[1..];
            res
        }
    };
}
#[macro_export]
macro_rules! write_string {
    ($buf:expr, $string:expr) => {
        write_u32!($buf, $string.len() as u32);
        $buf.write_all($string.as_bytes()).unwrap();
    };
}
#[macro_export]
macro_rules! read_string {
    ($buf:expr) => {
        {
            let string_length = read_u32!($buf) as usize;
            let res = String::from_utf8($buf[..string_length].to_vec())?;
            $buf = &$buf[string_length..];
            res
        }
    };
}
#[macro_export]
macro_rules! write_attrs {
    ($buf:expr, $attrs:expr) => {
        {
            let flags = Attrsflags {
                size: $attrs.size.is_some(),
                uidgid: $attrs.uid_gid.is_some(),
                permissions: $attrs.permissions.is_some(),
                acmodtime: $attrs.atime_mtime.is_some(),
                extended: $attrs.extended_attrs.len() > 0,
            };
            let flags_u32: u32 = flags.into();
            write_u32!($buf, flags_u32);
            if let Some(size) = $attrs.size {
                write_u64!($buf, size);
            }
            if let Some((uid, gid)) = $attrs.uid_gid {
                write_u32!($buf, uid);
                write_u32!($buf, gid);
            }
            if let Some(permissions) = $attrs.permissions {
                write_u32!($buf, permissions);
            }
            if let Some((atime, mtime)) = $attrs.atime_mtime {
                write_u32!($buf, atime);
                write_u32!($buf, mtime);
            }
            if $attrs.extended_attrs.len() > 0 {
                write_u32!($buf, $attrs.extended_attrs.len() as u32);
                for ext in &$attrs.extended_attrs {
                    write_string!($buf, ext.r#type);
                    write_string!($buf, ext.data);
                }
            }
        }
    };
}
#[macro_export]
macro_rules! read_attrs {
    ($buf:expr) => {
        {
            let flags: Attrsflags = read_u32!($buf).into();
            let mut res = Attrs::default();
            if flags.size {
                res.size = Some(read_u64!($buf));
            }
            if flags.uidgid {
                res.uid_gid = Some((read_u32!($buf), read_u32!($buf)));
            }
            if flags.permissions {
                res.permissions = Some(read_u32!($buf));
            }
            if flags.acmodtime {
                res.atime_mtime = Some((read_u32!($buf), read_u32!($buf)));
            }
            if flags.extended {
                let mut len = read_u32!($buf);
                while len > 0 {
                    res.extended_attrs.push(ExtendedAttr {
                        r#type: read_string!($buf),
                        data: read_string!($buf),
                    });
                    len -= 1;
                }
            }
            res
        }
    };
}

impl Into<u32> for StatusCode {
    fn into(self) -> u32 {
        match self {
            StatusCode::r#Ok => 0,
            StatusCode::Eof => 1,
            StatusCode::NoSuchFile => 2,
            StatusCode::PermissionDenied => 3,
            StatusCode::Failure => 4,
            StatusCode::BadMessage => 5,
            StatusCode::NoConnection => 6,
            StatusCode::ConnectionLost => 7,
            StatusCode::OpUnsupported => 8,
        }
    }
}

impl Into<u32> for Attrsflags {
    fn into(self) -> u32 {
        let mut res = 0;
        if self.size        { res +=                                0b1; }
        if self.uidgid      { res +=                               0b10; }
        if self.permissions { res +=                              0b100; }
        if self.acmodtime   { res +=                             0b1000; }
        if self.extended    { res += 0b10000000000000000000000000000000; }
        res
    }
}

impl From<u32> for Attrsflags {
    fn from(num: u32) -> Attrsflags {
        Attrsflags {
            size:        num &                                0b1 != 0,
            uidgid:      num &                               0b10 != 0,
            permissions: num &                              0b100 != 0,
            acmodtime:   num &                             0b1000 != 0,
            extended:    num & 0b10000000000000000000000000000000 != 0,
        }
    }
}

impl From<u32> for Pflags {
    fn from(num: u32) -> Pflags {
        Pflags {
            read:   num &      0b1 != 0,
            write:  num &     0b10 != 0,
            append: num &    0b100 != 0,
            creat:  num &   0b1000 != 0,
            trunc:  num &  0b10000 != 0,
            excl:   num & 0b100000 != 0,
        }
    }
}

pub(crate) fn statvfs_to_bytes(stat: libc::statvfs) -> Vec<u8> {
    let mut res = Vec::new();
    write_u64!(res, stat.f_bsize);
    write_u64!(res, stat.f_frsize);
    write_u64!(res, stat.f_blocks);
    write_u64!(res, stat.f_bfree);
    write_u64!(res, stat.f_bavail);
    write_u64!(res, stat.f_files);
    write_u64!(res, stat.f_ffree);
    write_u64!(res, stat.f_favail);
    write_u64!(res, stat.f_fsid);
    write_u64!(res, stat.f_flag);
    write_u64!(res, stat.f_namemax);
    res
}

impl SftpClientPacket {
    pub fn from_bytes(mut data: &[u8]) -> Result<SftpClientPacket, ProtocolError> {
        if data.len() < 4 {
            return Err(ProtocolError::IncompleteBuffer);
        }
        let len = read_u32!(data) as usize;
        if data.len() < len {
            return Err(ProtocolError::IncompleteBuffer);
        }
        data = &data[..len];
        let command = read_u8!(data);

        let res = match command {
            1 => {
                let version = read_u32!(data);

                let mut extensions = Vec::new();

                let mut strings = Vec::new();
                while data.len() > 0 {
                    strings.push(read_string!(data));
                }

                let pairs_iter = strings
                    .chunks_exact(2);

                for pair in pairs_iter {
                    extensions.push(Extension {
                        name: pair[0].clone(),
                        data: pair[1].clone(),
                    });
                }

                SftpClientPacket::Init { version, extensions }
            },
            3 => {
                SftpClientPacket::Open {
                    id: read_u32!(data),
                    filename: read_string!(data),
                    pflags: read_u32!(data).into(),
                    attrs: read_attrs!(data),
                }
            },
            4 => {
                SftpClientPacket::Close {
                    id: read_u32!(data),
                    handle: read_string!(data),
                }
            },
            5 => {
                SftpClientPacket::Read {
                    id: read_u32!(data),
                    handle: read_string!(data),
                    offset: read_u64!(data),
                    len: read_u32!(data),
                }
            },
            6 => {
                SftpClientPacket::Write {
                    id: read_u32!(data),
                    handle: read_string!(data),
                    offset: read_u64!(data),
                    data: {
                        let len = read_u32!(data) as usize;
                        //println!("write len {}", len);
                        let res = data[..len].to_vec();
                        data = &data[len..];
                        res
                    },
                }
            },
            7 => {
                SftpClientPacket::Lstat {
                    id: read_u32!(data),
                    path: read_string!(data),
                }
            },
            8 => {
                SftpClientPacket::Fstat {
                    id: read_u32!(data),
                    handle: read_string!(data),
                }
            },
            9 => {
                SftpClientPacket::Setstat {
                    id: read_u32!(data),
                    path: read_string!(data),
                    attrs: read_attrs!(data),
                }
            },
            10 => {
                SftpClientPacket::Fsetstat {
                    id: read_u32!(data),
                    handle: read_string!(data),
                    attrs: read_attrs!(data),
                }
            },
            11 => {
                SftpClientPacket::Opendir {
                    id: read_u32!(data),
                    path: read_string!(data),
                }
            },
            12 => {
                SftpClientPacket::Readdir {
                    id: read_u32!(data),
                    handle: read_string!(data),
                }
            },
            13 => {
                SftpClientPacket::Remove {
                    id: read_u32!(data),
                    filename: read_string!(data),
                }
            },
            14 => {
                SftpClientPacket::Mkdir {
                    id: read_u32!(data),
                    path: read_string!(data),
                    attrs: read_attrs!(data),
                }
            },
            15 => {
                SftpClientPacket::Rmdir {
                    id: read_u32!(data),
                    path: read_string!(data),
                }
            },
            16 => {
                SftpClientPacket::Realpath {
                    id: read_u32!(data),
                    path: read_string!(data),
                }
            },
            17 => {
                SftpClientPacket::Stat {
                    id: read_u32!(data),
                    path: read_string!(data),
                }
            },
            18 => {
                SftpClientPacket::Rename {
                    id: read_u32!(data),
                    oldpath: read_string!(data),
                    newpath: read_string!(data),
                }
            },
            19 => {
                SftpClientPacket::Readlink {
                    id: read_u32!(data),
                    path: read_string!(data),
                }
            },
            20 => {
                SftpClientPacket::Symlink {
                    id: read_u32!(data),
                    // these are switched to imitate a bug in openssh sftp-server
                    targetpath: read_string!(data),
                    linkpath: read_string!(data),
                }
            },
            200 => {
                SftpClientPacket::Extended {
                    id: read_u32!(data),
                    extended_request: read_string!(data),
                    data: {
                        let res = data.to_vec();
                        data = &[];
                        res
                    },
                }
            },
            _ => {
                // unknown command
                Err(ProtocolError::UnknownCommand)?
            },
        };

        if data.len() == 0 {
            Ok(res)
        } else {
            //println!("left: {}", data.len());
            Err(ProtocolError::InvalidLength)
        }
    }
}

impl SftpServerPacket {
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut res = Vec::new();
        write_u32!(res, 0u32); // length placeholder

        match self {
            SftpServerPacket::Version { version, extensions } => {
                write_u8!(res, 2u8);
                write_u32!(res, *version);
                for extension in extensions {
                    write_string!(res, extension.name);
                    write_string!(res, extension.data);
                }
            },
            SftpServerPacket::Status { id, status_code, error_message, language_tag } => {
                write_u8!(res, 101u8);
                write_u32!(res, *id);
                write_u32!(res, *status_code);
                write_string!(res, error_message);
                write_string!(res, language_tag);
            },
            SftpServerPacket::Handle { id, handle } => {
                write_u8!(res, 102u8);
                write_u32!(res, *id);
                write_string!(res, handle);
            },
            SftpServerPacket::Data { id, data } => {
                write_u8!(res, 103u8);
                write_u32!(res, *id);
                write_u32!(res, data.len() as u32);
                res.write_all(&data).unwrap();
            },
            SftpServerPacket::Name { id, names } => {
                write_u8!(res, 104u8);
                write_u32!(res, *id);
                write_u32!(res, names.len() as u32);
                for name in names {
                    write_string!(res, name.filename);
                    write_string!(res, name.longname);
                    write_attrs!(res, name.attrs);
                }
            },
            SftpServerPacket::Attrs { id, attrs } => {
                write_u8!(res, 105u8);
                write_u32!(res, *id);
                write_attrs!(res, attrs);
            },
            SftpServerPacket::ExtendedReply { id, data } => {
                write_u8!(res, 201u8);
                write_u32!(res, *id);
                res.write_all(&data).unwrap();
            },
        }

        let len_bytes = (res.len() as u32 - 4).to_be_bytes();
        res[0] = len_bytes[0]; res[1] = len_bytes[1]; res[2] = len_bytes[2]; res[3] = len_bytes[3];

        return Ok(res);
    }
}
