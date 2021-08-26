use crate::types::*;
use crate::error::Result;
use std::convert::TryInto;

pub trait Serialize {
    fn serialize(&self) -> anyhow::Result<Vec<u8>>;
}
pub trait Deserialize: Sized {
    fn deserialize(input: &mut &[u8]) -> anyhow::Result<Self>;
}

impl Serialize for u8 {
    fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        Ok(vec![*self; 1])
    }
}
impl Deserialize for u8 {
    fn deserialize(input: &mut &[u8]) -> anyhow::Result<Self> {
        let res = input[0];
        *input = &mut &input[1..];
        Ok(res)
    }
}

impl Serialize for u32 {
    fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }
}
impl Deserialize for u32 {
    fn deserialize(input: &mut &[u8]) -> anyhow::Result<Self> {
        let res = Self::from_be_bytes((input[..4]).try_into()?);
        *input = &mut &input[4..];
        Ok(res)
    }
}

impl Serialize for u64 {
    fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }
}
impl Deserialize for u64 {
    fn deserialize(input: &mut &[u8]) -> anyhow::Result<Self> {
        let res = Self::from_be_bytes((input[..8]).try_into()?);
        *input = &mut &input[8..];
        Ok(res)
    }
}

impl Serialize for String {
    fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        let mut res = Vec::new();
        let len = self.len() as u32;
        res.append(&mut len.serialize()?);
        res.append(&mut self.as_bytes().to_vec());
        Ok(res)
    }
}
impl Deserialize for String {
    fn deserialize(input: &mut &[u8]) -> anyhow::Result<Self> {
        let len = u32::deserialize(input)? as usize;
        let res = Self::from_utf8_lossy(&input[..len]);
        *input = &mut &input[len..];
        Ok(res.to_string())
    }
}

impl Serialize for Data {
    fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        let mut res = Vec::new();
        let len = self.0.len() as u32;
        res.append(&mut len.serialize()?);
        res.append(&mut self.0.clone());
        Ok(res)
    }
}
impl Deserialize for Data {
    fn deserialize(input: &mut &[u8]) -> anyhow::Result<Self> {
        let len = u32::deserialize(input)? as usize;
        Ok(Data(input[..len].to_vec()))
    }
}

impl<T> Serialize for Vec<T> where T: Serialize {
    fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        let mut res = Vec::new();
        let len = self.len() as u32;
        res.append(&mut len.serialize()?);
        for el in self {
            res.append(&mut el.serialize()?);
        }
        Ok(res)
    }
}
impl<T> Deserialize for Vec<T> where T: Deserialize {
    fn deserialize(input: &mut &[u8]) -> anyhow::Result<Self> {
        let mut res = Vec::new();
        let len = u32::deserialize(input)? as usize;
        for _i in 0..len {
            res.push(Deserialize::deserialize(input)?);
        }
        Ok(res)
    }
}

impl Serialize for Attrs {
    fn serialize(&self) -> Result<Vec<u8>> {
        let mut res = Vec::new();
        let flags = Attrsflags {
            size: self.size.is_some(),
            uidgid: self.uid_gid.is_some(),
            permissions: self.permissions.is_some(),
            acmodtime: self.atime_mtime.is_some(),
            extended: self.extended_attrs.len() > 0,
        };
        res.append(&mut flags.serialize()?);
        if let Some(size) = self.size {
            res.append(&mut size.serialize()?);
        }
        if let Some((uid, gid)) = self.uid_gid {
            res.append(&mut uid.serialize()?);
            res.append(&mut gid.serialize()?);
        }
        if let Some(permissions) = self.permissions {
            res.append(&mut permissions.serialize()?);
        }
        if let Some((atime, mtime)) = self.atime_mtime {
            res.append(&mut atime.serialize()?);
            res.append(&mut mtime.serialize()?);
        }
        if self.extended_attrs.len() > 0 {
            res.append(&mut self.extended_attrs.serialize()?);
        }
        Ok(res)
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
    fn serialize(&self) -> Result<Vec<u8>> {
        let mut num = 0u32;
        if self.size        { num +=                                0b1; }
        if self.uidgid      { num +=                               0b10; }
        if self.permissions { num +=                              0b100; }
        if self.acmodtime   { num +=                             0b1000; }
        if self.extended    { num += 0b10000000000000000000000000000000; }
        num.serialize()
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
    fn serialize(&self) -> Result<Vec<u8>> {
        let mut num = 0u32;
        if self.read   { num +=      0b1; }
        if self.write  { num +=     0b10; }
        if self.append { num +=    0b100; }
        if self.creat  { num +=   0b1000; }
        if self.trunc  { num +=  0b10000; }
        if self.excl   { num += 0b100000; }
        num.serialize()
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

