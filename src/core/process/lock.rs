use super::encoding::safe_command_output_to_string;
/// File lock detection module
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileLockProcess {
    pub pid: u32,
    pub name: String,
    pub cmd: String,
}

#[derive(Debug, Clone)]
pub struct FileLockInfo {
    pub path: PathBuf,
    pub locked: bool,
    pub processes: Vec<FileLockProcess>,
}

/// Detect if a file is locked by a process
pub fn is_file_locked(path: &Path) -> bool {
    // If file doesn't exist, it's not considered locked
    if !path.exists() {
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        is_file_locked_windows(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        is_file_locked_unix(path)
    }
}

/// Windows file lock detection
#[cfg(target_os = "windows")]
fn is_file_locked_windows(path: &Path) -> bool {
    use std::fs::OpenOptions;
    use std::io::ErrorKind;

    // For directories, use a different detection method
    if path.is_dir() {
        return is_directory_locked(path);
    }

    // Try to open file in write mode, but analyze error type more precisely
    match OpenOptions::new().write(true).create(false).open(path) {
        Ok(_) => false,
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => check_file_locking_status(path),
            ErrorKind::NotFound => false,
            _ => {
                eprintln!(
                    "Warning: file open failed, may be in use: {} - {}",
                    path.display(),
                    e
                );
                true
            }
        },
    }
}

/// Unix file lock detection
#[cfg(not(target_os = "windows"))]
fn is_file_locked_unix(path: &Path) -> bool {
    let path_str = match path.to_str() {
        Some(s) => s,
        None => return false,
    };

    match std::process::Command::new("lsof").arg(path_str).output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Windows-specific: check if a directory is locked
#[cfg(target_os = "windows")]
fn is_directory_locked(path: &Path) -> bool {
    // First check basic directory access permissions
    match std::fs::read_dir(path) {
        Ok(entries) => {
            // Use RestartManager to register directory path detection
            if let Ok(pids) = find_processes_with_restart_manager(path) {
                if !pids.is_empty() {
                    return true;
                }
            }

            // Sample first batch of child files in directory for lock detection
            for entry in entries.take(10).flatten() {
                let child = entry.path();
                let metadata = match child.symlink_metadata() {
                    Ok(m) => m,
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return true,
                    Err(_) => continue,
                };
                if !metadata.is_dir() && !metadata.file_type().is_symlink() {
                    if let Ok(pids) = find_processes_with_restart_manager(&child) {
                        if !pids.is_empty() {
                            return true;
                        }
                    }
                }
            }
            false
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                eprintln!(
                    "Warning: directory access denied, may be locked: {}",
                    path.display()
                );
                true
            }
            _ => {
                eprintln!("Warning: directory read failed: {} - {}", path.display(), e);
                true
            }
        },
    }
}

/// Escape path as PowerShell single-quoted string (' → '')
#[cfg(target_os = "windows")]
fn ps_escape(s: &str) -> String {
    s.replace('\'', "''")
}

/// Windows-specific: more precise file lock status check
#[cfg(target_os = "windows")]
fn check_file_locking_status(path: &Path) -> bool {
    use std::fs::OpenOptions;

    // Method 1: try opening in read-only mode
    if OpenOptions::new().read(true).open(path).is_ok() {
        // Can open read-only but not write, most likely locked
        return true;
    }

    // Method 2: try opening in append mode (doesn't modify content but needs write permission)
    match OpenOptions::new().append(true).open(path) {
        Ok(_) => false,
        Err(e) => match e.kind() {
            std::io::ErrorKind::PermissionDenied => true,
            _ => {
                // Other errors, use PowerShell as supplementary detection
                let escaped = ps_escape(&path.to_string_lossy());
                match std::process::Command::new("powershell")
                    .args([
                        "-Command",
                        &format!(
                            "Get-Process | Where-Object {{$_.Modules.FileName -eq '{escaped}'}} | Measure-Object"
                        ),
                    ])
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        let output_str = String::from_utf8_lossy(&output.stdout);
                        !output_str.trim().contains("0")
                    }
                    _ => true,
                }
            }
        },
    }
}

/// Windows-specific: use RestartManager API to find processes holding a file
/// This is the same API used by Windows Explorer, providing precise file handle detection
#[cfg(target_os = "windows")]
fn find_processes_with_restart_manager(path: &Path) -> Result<Vec<u32>> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::RestartManager::*;

    let mut pids = Vec::new();

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut session_handle: u32 = 0;
    let mut session_key: [u16; 1] = [0];

    let result = unsafe { RmStartSession(&mut session_handle, 0, session_key.as_mut_ptr()) };

    if result != 0 {
        return Ok(pids);
    }

    // RAII guard to ensure session cleanup
    struct RmSession {
        handle: u32,
    }
    impl Drop for RmSession {
        fn drop(&mut self) {
            unsafe {
                RmEndSession(self.handle);
            }
        }
    }
    let _session = RmSession {
        handle: session_handle,
    };

    let result = unsafe {
        RmRegisterResources(
            session_handle,
            1,
            &wide_path.as_ptr(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
        )
    };

    if result != 0 {
        return Ok(pids);
    }

    let mut proc_info_needed: u32 = 0;
    let mut proc_info_count: u32 = 0;
    let mut reboot_reasons: u32 = 0;

    let result = unsafe {
        RmGetList(
            session_handle,
            &mut proc_info_needed,
            &mut proc_info_count,
            std::ptr::null_mut(),
            &mut reboot_reasons,
        )
    };

    if result != 233 && result != 0 {
        return Ok(pids);
    }

    if proc_info_needed == 0 {
        return Ok(pids);
    }

    let mut process_info: Vec<RM_PROCESS_INFO> =
        vec![unsafe { std::mem::zeroed() }; proc_info_needed as usize];
    proc_info_count = proc_info_needed;

    let result = unsafe {
        RmGetList(
            session_handle,
            &mut proc_info_needed,
            &mut proc_info_count,
            process_info.as_mut_ptr(),
            &mut reboot_reasons,
        )
    };

    if result != 0 {
        return Ok(pids);
    }

    for info in process_info.iter().take(proc_info_count as usize) {
        let pid = info.Process.dwProcessId;
        if pid != 0 && !pids.contains(&pid) {
            pids.push(pid);
        }
    }

    Ok(pids)
}

/// Find processes locking a specified file
#[cfg(target_os = "windows")]
pub fn find_processes_by_file(path: &Path) -> Result<Vec<u32>> {
    let mut pids = Vec::new();

    if !path.exists() {
        return Ok(pids);
    }

    let path_str = path.to_string_lossy();

    // Method 1 (preferred): use RestartManager API for precise file handle detection
    if let Ok(rm_pids) = find_processes_with_restart_manager(path) {
        pids.extend(rm_pids);
    }

    // Method 2: use handle.exe tool (if available)
    if let Ok(handle_pids) = find_processes_with_handle(&path_str) {
        for pid in handle_pids {
            if !pids.contains(&pid) {
                pids.push(pid);
            }
        }
    }

    // Method 3: use PowerShell to find (compatibility fallback)
    if let Ok(ps_pids) = find_processes_with_powershell(&path_str) {
        for pid in ps_pids {
            if !pids.contains(&pid) {
                pids.push(pid);
            }
        }
    }

    Ok(pids)
}

/// Find processes locking a specified file
#[cfg(not(target_os = "windows"))]
pub fn find_processes_by_file(path: &Path) -> Result<Vec<u32>> {
    let mut pids = Vec::new();

    if !path.exists() {
        return Ok(pids);
    }

    let path_str = match path.to_str() {
        Some(s) => s,
        None => return Ok(pids),
    };

    if let Ok(output) = std::process::Command::new("lsof")
        .arg("-t")
        .arg(path_str)
        .output()
    {
        if output.status.success() {
            let output_str = safe_command_output_to_string(&output.stdout);
            for line in output_str.lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    pids.push(pid);
                }
            }
        }
    }

    Ok(pids)
}

/// Windows-specific: use handle.exe to find locking processes
#[cfg(target_os = "windows")]
fn find_processes_with_handle(path_str: &str) -> Result<Vec<u32>> {
    let mut pids = Vec::new();

    match std::process::Command::new("handle.exe")
        .arg(path_str)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                let output_str = safe_command_output_to_string(&output.stdout);
                for line in output_str.lines() {
                    // handle.exe output format is typically: pid: process_name path
                    if line.contains("pid:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2
                            && let Some(pid_part) = parts.get(1)
                            && let Ok(pid) = pid_part.trim_end_matches(':').parse::<u32>()
                        {
                            pids.push(pid);
                        }
                    }
                }
            }
        }
        Err(_) => {
            // handle.exe not available, this is normal
        }
    }

    Ok(pids)
}

/// Windows-specific: use PowerShell to find locking processes
#[cfg(target_os = "windows")]
fn find_processes_with_powershell(path_str: &str) -> Result<Vec<u32>> {
    let mut pids = Vec::new();

    let escaped = ps_escape(path_str);

    let powershell_commands = vec![
        format!(
            "Get-Process | Where-Object {{$_.MainModule.FileName -like '*{}*'}} | Select-Object -ExpandProperty Id",
            escaped
        ),
        format!(
            "$path = '{}'; Get-Process | ForEach-Object {{ if ($_.Modules.FileName -contains $path) {{ $_.Id }} }}",
            escaped
        ),
    ];

    for command in powershell_commands {
        match std::process::Command::new("powershell")
            .args(["-Command", &command])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let output_str = safe_command_output_to_string(&output.stdout);
                    for line in output_str.lines() {
                        let trimmed = line.trim();
                        if trimmed.chars().all(|c| c.is_ascii_digit())
                            && !trimmed.is_empty()
                            && let Ok(pid) = trimmed.parse::<u32>()
                            && !pids.contains(&pid)
                        {
                            pids.push(pid);
                        }
                    }
                }
            }
            Err(_) => {
                // PowerShell command failed, continue to the next one
                continue;
            }
        }
    }

    Ok(pids)
}
