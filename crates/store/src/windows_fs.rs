use crate::records::storage;
use mstrchf_contracts::DomainError;
use std::{fs::File, path::Path};

#[cfg(windows)]
pub(crate) fn identity(file: &File) -> Result<String, DomainError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };
    let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // The handle stays open for this query and the OS initialises info on success.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) } == 0 {
        return Err(storage(std::io::Error::last_os_error()));
    }
    let info = unsafe { info.assume_init() };
    Ok(format!(
        "{}:{}:{}",
        info.dwVolumeSerialNumber, info.nFileIndexHigh, info.nFileIndexLow
    ))
}
#[cfg(windows)]
pub(crate) fn publish(source: &Path, target: &Path) -> Result<(), DomainError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};
    let from: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    // Deliberately omit REPLACE_EXISTING: existing unrelated data must survive.
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
        return Err(storage(std::io::Error::last_os_error()));
    }
    Ok(())
}
#[cfg(not(windows))]
pub(crate) fn identity(_: &File) -> Result<String, DomainError> {
    Err(crate::records::unsupported())
}
#[cfg(not(windows))]
pub(crate) fn publish(_: &Path, _: &Path) -> Result<(), DomainError> {
    Err(crate::records::unsupported())
}
