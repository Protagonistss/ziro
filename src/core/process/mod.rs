//! Process management module
//!
//! Provides process querying, termination, and file lock detection capabilities

use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

// Export submodules
pub mod encoding;
pub mod lock;

// Re-export commonly used types and functions
pub use lock::{FileLockInfo, FileLockProcess, find_processes_by_file, is_file_locked};

/// Kill the process with the given PID
pub fn kill_process(pid: u32) -> Result<()> {
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_all();

    let pid_obj = sysinfo::Pid::from_u32(pid);

    if let Some(process) = sys.process(pid_obj) {
        if process.kill() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to kill process {pid} (administrator privileges may be required)"
            ))
        }
    } else {
        Err(anyhow!("Process {pid} does not exist"))
    }
}

/// Kill multiple processes
pub fn kill_processes(pids: &[u32]) -> Vec<(u32, Result<()>)> {
    pids.iter().map(|&pid| (pid, kill_process(pid))).collect()
}

/// Force kill the process with the given PID (multiple attempts)
pub fn kill_process_force(pid: u32) -> Result<()> {
    // First check if the process exists
    {
        let mut sys = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
        );
        sys.refresh_all();

        let pid_obj = sysinfo::Pid::from_u32(pid);
        if sys.process(pid_obj).is_none() {
            // Process no longer exists, consider it a success
            return Ok(());
        }
    }

    // Attempt to kill the process up to 3 times
    for attempt in 1..=3 {
        {
            let mut sys = System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
            );
            sys.refresh_all();

            let pid_obj = sysinfo::Pid::from_u32(pid);
            if let Some(process) = sys.process(pid_obj) {
                if process.kill() {
                    // Wait for the process to actually exit
                    thread::sleep(Duration::from_millis(500));

                    // Refresh process status and check if it still exists
                    sys.refresh_processes(sysinfo::ProcessesToUpdate::All);
                    if !sys.processes().contains_key(&pid_obj) {
                        return Ok(());
                    }
                } else {
                    // If kill() returns false
                    if attempt == 3 {
                        return Err(anyhow!(
                            "Failed to force kill process {pid} (administrator privileges may be required)"
                        ));
                    }
                }
            } else {
                // Process no longer exists, consider it a success
                return Ok(());
            }
        }

        // If not the last attempt, wait before retrying
        if attempt < 3 {
            thread::sleep(Duration::from_millis(1000));
        }
    }

    Err(anyhow!(
        "Force kill of process {pid} failed, the process may still be running"
    ))
}

/// Force kill multiple processes
pub fn kill_processes_force(pids: &[u32]) -> Vec<(u32, Result<()>)> {
    pids.iter()
        .map(|&pid| (pid, kill_process_force(pid)))
        .collect()
}

/// Check file lock status
pub fn inspect_file_locks(paths: &[PathBuf]) -> Result<Vec<FileLockInfo>> {
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_all();

    let mut results = Vec::new();

    for path in paths {
        let mut locked = is_file_locked(path);
        let mut pids = find_processes_by_file(path).unwrap_or_default();
        pids.sort_unstable();
        pids.dedup();

        let mut processes = Vec::new();
        for pid in pids {
            if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
                let name = process.name().to_string_lossy().to_string();
                let cmd = process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().to_string())
                    .collect::<Vec<String>>()
                    .join(" ");
                processes.push(FileLockProcess { pid, name, cmd });
            } else {
                processes.push(FileLockProcess {
                    pid,
                    name: "unknown".to_string(),
                    cmd: String::new(),
                });
            }
        }

        if !processes.is_empty() {
            locked = true;
        }

        results.push(FileLockInfo {
            path: path.clone(),
            locked,
            processes,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_inspect_file_locks_for_temp_file() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!(
            "ziro_lock_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));

        fs::write(&file_path, b"test").unwrap();

        let infos = inspect_file_locks(std::slice::from_ref(&file_path)).unwrap();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].path, file_path);

        if infos[0].processes.is_empty() {
            assert!(!infos[0].locked);
        } else {
            assert!(infos[0].locked);
        }

        let _ = fs::remove_file(&file_path);
    }
}
