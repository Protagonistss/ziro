use anyhow::{Result, anyhow};
use std::path::Path;
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

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

/// 检测文件是否被进程占用
pub fn is_file_locked(path: &Path) -> bool {
    // 如果文件不存在，不算被占用
    if !path.exists() {
        return false;
    }

    // 在 Windows 上，尝试以写入模式打开文件来判断是否被占用
    if cfg!(target_os = "windows") {
        use std::fs::OpenOptions;

        // 尝试以写入模式打开文件
        match OpenOptions::new().write(true).create(false).open(path) {
            Ok(_) => {
                // 能够正常打开，说明文件没有被占用
                // 文件句柄会在函数结束时自动关闭
                false
            }
            Err(_) => {
                // 无法打开，可能被占用
                // 注意：这里也可能是权限问题，为了安全起见，我们假设被占用
                true
            }
        }
    } else {
        // 在 Unix 系统上，使用 lsof 命令来检测文件是否被占用
        let path_str = match path.to_str() {
            Some(s) => s,
            None => return false,
        };

        match std::process::Command::new("lsof").arg(path_str).output() {
            Ok(output) => output.status.success(),
            Err(_) => false, // lsof 命令不可用，假设没有被占用
        }
    }
}

/// 查找占用指定文件的进程
pub fn find_processes_by_file(path: &Path) -> Result<Vec<u32>> {
    let mut pids = Vec::new();

    if !path.exists() {
        return Ok(pids);
    }

    if cfg!(target_os = "windows") {
        // Windows 系统的实现
        // 使用 handle.exe 或 powershell 命令来查找占用文件的进程
        let path_str = path.to_string_lossy();

        // 使用 PowerShell 查找占用文件的进程
        match std::process::Command::new("powershell")
            .args([
                "-Command",
                &format!("Get-Process | Where-Object {{ $_.MainModule.FileName -eq '{path_str}' }} | Select-Object Id")
            ])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        if let Ok(pid) = line.trim().parse::<u32>() {
                            pids.push(pid);
                        }
                    }
                }
            }
            Err(_) => {
                // PowerShell 不可用，尝试其他方法
                // 这里可以简化处理，假设无法获取进程信息
            }
        }
    } else {
        // Unix 系统的实现
        let path_str = match path.to_str() {
            Some(s) => s,
            None => return Ok(pids),
        };

        match std::process::Command::new("lsof")
            .arg("-t")
            .arg(path_str)
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        if let Ok(pid) = line.trim().parse::<u32>() {
                            pids.push(pid);
                        }
                    }
                }
            }
            Err(_) => {
                // lsof 命令不可用
            }
        }
    }

    Ok(pids)
}
