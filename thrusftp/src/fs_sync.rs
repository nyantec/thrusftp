use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::ffi::CString;
use std::path::Path;
use std::convert::TryInto;
use std::io::{Result, Error, ErrorKind};

pub(crate) fn statvfs<P: AsRef<Path>>(path: P) -> Result<libc::statvfs> {
    let cstr = match CString::new(path.as_ref().as_os_str().as_bytes()) {
        Ok(cstr) => cstr,
        Err(..) => return Err(Error::new(ErrorKind::InvalidInput, "path contained a null")),
    };

	let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::zeroed();

	if unsafe { libc::statvfs(cstr.as_ptr(), stat.as_mut_ptr()) } != 0 {
		Err(Error::last_os_error())
	} else {
		let stat = unsafe { stat.assume_init() };
        Ok(stat)
	}
}

pub(crate) fn truncate64<P: AsRef<Path>>(path: P, size: u64) -> Result<()> {
    let cstr = CString::new(path.as_ref().as_os_str().as_bytes())?;
    let size = size.try_into().map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;

    if unsafe { libc::truncate64(cstr.as_ptr(), size) } != 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}
