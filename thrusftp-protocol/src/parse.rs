use std::io::Write;
use crate::types::*;
use anyhow::Result;
use std::convert::TryInto;

pub trait Serialize {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()>;
}
pub trait Deserialize: Sized {
    fn deserialize(input: &mut &[u8]) -> Result<Self>;
}

impl Serialize for u8 {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        writer.write_all(&[*self; 1])?;
        Ok(())
    }
}
impl Deserialize for u8 {
    fn deserialize(input: &mut &[u8]) -> Result<Self> {
        let res = input[0];
        *input = &mut &input[1..];
        Ok(res)
    }
}

impl Serialize for u32 {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}
impl Deserialize for u32 {
    fn deserialize(input: &mut &[u8]) -> Result<Self> {
        let res = Self::from_be_bytes((input[..4]).try_into()?);
        *input = &mut &input[4..];
        Ok(res)
    }
}

impl Serialize for u64 {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}
impl Deserialize for u64 {
    fn deserialize(input: &mut &[u8]) -> Result<Self> {
        let res = Self::from_be_bytes((input[..8]).try_into()?);
        *input = &mut &input[8..];
        Ok(res)
    }
}

impl Serialize for String {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        let len = self.len() as u32;
        len.serialize(writer)?;
        writer.write_all(self.as_bytes())?;
        Ok(())
    }
}
impl Deserialize for String {
    fn deserialize(input: &mut &[u8]) -> Result<Self> {
        let len = u32::deserialize(input)? as usize;
        let res = String::from_utf8((&input[..len]).to_vec())?;
        *input = &mut &input[len..];
        Ok(res.to_string())
    }
}

impl Serialize for VecU8 {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        let len = self.0.len() as u32;
        len.serialize(writer)?;
        writer.write_all(&self.0)?;
        Ok(())
    }
}
impl Deserialize for VecU8 {
    fn deserialize(input: &mut &[u8]) -> Result<Self> {
        let len = u32::deserialize(input)? as usize;
        Ok(VecU8(input[..len].to_vec()))
    }
}

impl<T> Serialize for Vec<T> where T: Serialize {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        let len = self.len() as u32;
        len.serialize(writer)?;
        for el in self {
            el.serialize(writer)?;
        }
        Ok(())
    }
}
impl<T> Deserialize for Vec<T> where T: Deserialize {
    fn deserialize(input: &mut &[u8]) -> Result<Self> {
        let mut res = Vec::new();
        let len = u32::deserialize(input)? as usize;
        for _i in 0..len {
            res.push(Deserialize::deserialize(input)?);
        }
        Ok(res)
    }
}

impl Serialize for Attrs {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        let flags = Attrsflags {
            size: self.size.is_some(),
            uidgid: self.uid_gid.is_some(),
            permissions: self.permissions.is_some(),
            acmodtime: self.atime_mtime.is_some(),
            extended: self.extended_attrs.len() > 0,
        };
        flags.serialize(writer)?;
        if let Some(size) = self.size {
            size.serialize(writer)?;
        }
        if let Some((uid, gid)) = self.uid_gid {
            uid.serialize(writer)?;
            gid.serialize(writer)?;
        }
        if let Some(permissions) = self.permissions {
            permissions.serialize(writer)?;
        }
        if let Some((atime, mtime)) = self.atime_mtime {
            atime.serialize(writer)?;
            mtime.serialize(writer)?;
        }
        if self.extended_attrs.len() > 0 {
            self.extended_attrs.serialize(writer)?;
        }
        Ok(())
    }
}

impl Deserialize for Attrs {
    fn deserialize(input: &mut &[u8]) -> Result<Self> {
        let flags: Attrsflags = Deserialize::deserialize(input)?;
        let mut res = Attrs::default();
        if flags.size {
            res.size = Some(Deserialize::deserialize(input)?);
        }
        if flags.uidgid {
            res.uid_gid = Some((
                Deserialize::deserialize(input)?,
                Deserialize::deserialize(input)?
            ));
        }
        if flags.permissions {
            res.permissions = Some(Deserialize::deserialize(input)?);
        }
        if flags.acmodtime {
            res.atime_mtime = Some((
                Deserialize::deserialize(input)?,
                Deserialize::deserialize(input)?
            ));
        }
        if flags.extended {
            res.extended_attrs = Deserialize::deserialize(input)?;
        }
        Ok(res)
    }
}

impl Serialize for Attrsflags {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        let mut num = 0u32;
        if self.size        { num +=                                0b1; }
        if self.uidgid      { num +=                               0b10; }
        if self.permissions { num +=                              0b100; }
        if self.acmodtime   { num +=                             0b1000; }
        if self.extended    { num += 0b10000000000000000000000000000000; }
        num.serialize(writer)
    }
}

impl Deserialize for Attrsflags {
    fn deserialize(input: &mut &[u8]) -> Result<Self> {
        u32::deserialize(input).map(|num| {
            Attrsflags {
                size:        num &                                0b1 != 0,
                uidgid:      num &                               0b10 != 0,
                permissions: num &                              0b100 != 0,
                acmodtime:   num &                             0b1000 != 0,
                extended:    num & 0b10000000000000000000000000000000 != 0,
            }
        })
    }
}

impl Serialize for Pflags {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        let mut num = 0u32;
        if self.read   { num +=      0b1; }
        if self.write  { num +=     0b10; }
        if self.append { num +=    0b100; }
        if self.creat  { num +=   0b1000; }
        if self.trunc  { num +=  0b10000; }
        if self.excl   { num += 0b100000; }
        num.serialize(writer)
    }
}

impl Deserialize for Pflags {
    fn deserialize(input: &mut &[u8]) -> Result<Self> {
        u32::deserialize(input).map(|num| {
            Pflags {
                read:   num &      0b1 != 0,
                write:  num &     0b10 != 0,
                append: num &    0b100 != 0,
                creat:  num &   0b1000 != 0,
                trunc:  num &  0b10000 != 0,
                excl:   num & 0b100000 != 0,
            }
        })
    }
}

impl Serialize for ExtendedRequestType {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        let s = match self {
            ExtendedRequestType::OpensshStatvfs => "statvfs@openssh.com",
            ExtendedRequestType::OpensshPosixRename => "posix-rename@openssh.com",
            ExtendedRequestType::OpensshHardlink => "hardlink@openssh.com",
            ExtendedRequestType::OpensshFsync => "fsync@openssh.com",
        };
        s.to_string().serialize(writer)
    }
}

impl Deserialize for ExtendedRequestType {
    fn deserialize(input: &mut &[u8]) -> Result<Self> {
        String::deserialize(input).map(|s| match s.as_str() {
            "statvfs@openssh.com" => ExtendedRequestType::OpensshStatvfs,
            "posix-rename@openssh.com" => ExtendedRequestType::OpensshPosixRename,
            "hardlink@openssh.com" => ExtendedRequestType::OpensshHardlink,
            "fsync@openssh.com" => ExtendedRequestType::OpensshFsync,
            _ => panic!("unexpected extended request"),
        })
    }
}

impl<T> Serialize for VecEos<T> where T: Serialize {
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        for ext in &self.0 {
            ext.serialize(writer)?;
        }
        Ok(())
    }
}
impl<T> Deserialize for VecEos<T> where T: Deserialize {
    fn deserialize(input: &mut &[u8]) -> Result<Self> {
        let mut res = Vec::new();
        while input.len() > 0 {
            res.push(T::deserialize(input)?);
        }
        Ok(res.into())
    }
}
