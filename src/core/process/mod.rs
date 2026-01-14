//! 进程管理模块
//!
//! 提供进程查询、终止、文件锁定检测等功能

use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

// 导出子模块
pub mod encoding;
pub mod lock;

// 重新导出常用类型和函数
pub use lock::{FileLockInfo, FileLockProcess, find_processes_by_file, is_file_locked};

/// 终止指定 PID 的进程
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
            Err(anyhow!("无法终止进程 {pid} (可能需要管理员权限)"))
        }
    } else {
        Err(anyhow!("进程 {pid} 不存在"))
    }
}

/// 批量终止进程
pub fn kill_processes(pids: &[u32]) -> Vec<(u32, Result<()>)> {
    pids.iter().map(|&pid| (pid, kill_process(pid))).collect()
}

/// 强制终止指定 PID 的进程（多次尝试）
pub fn kill_process_force(pid: u32) -> Result<()> {
    // 首先检查进程是否存在
    {
        let mut sys = System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
        );
        sys.refresh_all();

        let pid_obj = sysinfo::Pid::from_u32(pid);
        if sys.process(pid_obj).is_none() {
            // 进程已经不存在了，认为是成功的
            return Ok(());
        }
    }

    // 尝试最多 3 次终止进程
    for attempt in 1..=3 {
        {
            let mut sys = System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
            );
            sys.refresh_all();

            let pid_obj = sysinfo::Pid::from_u32(pid);
            if let Some(process) = sys.process(pid_obj) {
                if process.kill() {
                    // 等待进程真正退出
                    thread::sleep(Duration::from_millis(500));

                    // 刷新进程状态并检查是否是否存在
                    sys.refresh_processes(sysinfo::ProcessesToUpdate::All);
                    if !sys.processes().contains_key(&pid_obj) {
                        return Ok(());
                    }
                } else {
                    // 如果 kill() 返回 false
                    if attempt == 3 {
                        return Err(anyhow!("无法强制终止进程 {pid} (可能需要管理员权限)"));
                    }
                }
            } else {
                // 进程已经不存在了，认为是成功的
                return Ok(());
            }
        }

        // 如果不是最后一次尝试，等待一段时间后重试
        if attempt < 3 {
            thread::sleep(Duration::from_millis(1000));
        }
    }

    Err(anyhow!("强制终止进程 {pid} 失败，进程可能仍在运行"))
}

/// 批量强制终止进程
pub fn kill_processes_force(pids: &[u32]) -> Vec<(u32, Result<()>)> {
    pids.iter()
        .map(|&pid| (pid, kill_process_force(pid)))
        .collect()
}

/// 检查文件占用情况
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
