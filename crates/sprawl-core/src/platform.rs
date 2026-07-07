use crate::Result;
use std::path::PathBuf;

/// Set the current process/thread to the lowest OS scheduling priority.
/// This is how background work "yields" on all platforms.
pub fn set_low_priority() -> Result<()> {
    #[cfg(unix)]
    {
        // nice(19) — lowest priority on Linux/macOS
        unsafe {
            libc::nice(19);
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        // IDLE_PRIORITY_CLASS
        use windows_sys::Win32::System::Threading::*;
        unsafe {
            let process = GetCurrentProcess();
            SetPriorityClass(process, IDLE_PRIORITY_CLASS);
        }
        Ok(())
    }
}

/// Get the Sprawl data directory: ~/.sprawl/
pub fn sprawl_data_dir() -> Result<PathBuf> {
    // In a real app we'd use the `dirs` crate
    // For this scaffold we'll use a temp/dummy or standard rust std::env::var
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| crate::SprawlError::Other("Cannot determine home directory".into()))?;
    Ok(PathBuf::from(home).join(".sprawl"))
}
