//! Disk space monitoring utilities

use std::path::Path;

/// Get disk usage percentage for the given path
///
/// Returns the percentage of disk space used (0.0 - 100.0)
#[cfg(unix)]
pub fn get_disk_usage_percent(path: &Path) -> std::io::Result<f32> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let path_cstr = CString::new(path.as_os_str().as_bytes())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();

    let result = unsafe { libc::statvfs(path_cstr.as_ptr(), stat.as_mut_ptr()) };

    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }

    let stat = unsafe { stat.assume_init() };

    let block_size = stat.f_frsize as u64;
    let total_blocks = stat.f_blocks as u64;
    let available_blocks = stat.f_bavail as u64;

    let total = total_blocks * block_size;
    let available = available_blocks * block_size;

    if total == 0 {
        return Ok(0.0);
    }

    let used = total - available;
    Ok((used as f32 / total as f32) * 100.0)
}

/// Fallback for non-Unix systems
#[cfg(not(unix))]
pub fn get_disk_usage_percent(_path: &Path) -> std::io::Result<f32> {
    // On non-Unix systems, return 0% as a fallback
    // This could be extended with Windows-specific implementations
    Ok(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_get_disk_usage() {
        let path = PathBuf::from(".");
        let result = get_disk_usage_percent(&path);
        assert!(result.is_ok());
        let usage = result.unwrap();
        assert!(usage >= 0.0 && usage <= 100.0);
    }
}
