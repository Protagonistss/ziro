use anyhow::{Result, anyhow};
use std::path::Path;
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

/// 安全地转换命令输出为字符串，尝试多种编码方式
fn safe_command_output_to_string(stdout: &[u8]) -> String {
    // 首先尝试 UTF-8
    if let Ok(text) = std::str::from_utf8(stdout) {
        return text.to_string();
    }

    // 如果 UTF-8 失败，尝试检测 Windows 代码页
    #[cfg(target_os = "windows")]
    {
        // 尝试常见的中文编码
        if let Some(text) = try_decode_as_gbk(stdout) {
            return text;
        }

        // 尝试 Windows-1252
        if let Some(text) = try_decode_as_windows_1252(stdout) {
            return text;
        }
    }

    // 最后的回退：使用 lossy 转换，但记录错误
    let lossy = String::from_utf8_lossy(stdout);
    if lossy.contains('�') {
        // 如果有替换字符，说明有编码问题，但这在系统命令输出中很常见
        // 记录到 stderr 而不是 stdout，避免干扰程序输出
        eprintln!("警告: 命令输出包含非 UTF-8 字符，可能影响显示效果");
    }
    lossy.to_string()
}

#[cfg(target_os = "windows")]
fn try_decode_as_gbk(data: &[u8]) -> Option<String> {
    // 简化的 GBK 检测和转换
    // 这是一个基本实现，实际项目中可能需要使用 encoding crate
    if data.len() >= 2 {
        // 检查是否可能是 GBK 编码
        let mut valid_gbk = true;
        let mut i = 0;
        while i < data.len() - 1 {
            if data[i] >= 0x81 && data[i] <= 0xFE && data[i + 1] >= 0x40 && data[i + 1] <= 0xFE {
                // 可能是 GBK 字符
                i += 2;
            } else if data[i] <= 0x7F {
                // ASCII 字符
                i += 1;
            } else {
                valid_gbk = false;
                break;
            }
        }

        if valid_gbk {
            // 简单的 GBK 到 UTF-8 转换占位符
            // 实际实现需要使用适当的编码库
            return Some(format!("[GBK编码数据，长度: {}]", data.len()));
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn try_decode_as_windows_1252(data: &[u8]) -> Option<String> {
    // Windows-1252 检测
    // 检查是否包含有效的 Windows-1252 字符
    for &byte in data {
        if byte == 0x81 || byte == 0x8D || byte == 0x8F || byte == 0x90 || byte == 0x9D {
            // 这些是 Windows-1252 中的控制字符，在 UTF-8 中无效
            return Some(format!("[Windows-1252编码数据，长度: {}]", data.len()));
        }
    }
    None
}

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
                    let output_str = safe_command_output_to_string(&output.stdout);
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
                    let output_str = safe_command_output_to_string(&output.stdout);
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
