use super::encoding::safe_command_output_to_string;
/// 文件锁定检测模块
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

/// 检测文件是否被进程占用
pub fn is_file_locked(path: &Path) -> bool {
    // 如果文件不存在，不算被占用
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

/// Windows 文件锁定检测
#[cfg(target_os = "windows")]
fn is_file_locked_windows(path: &Path) -> bool {
    use std::fs::OpenOptions;
    use std::io::ErrorKind;

    // 如果是目录，使用不同的检测方法
    if path.is_dir() {
        return is_directory_locked(path);
    }

    // 尝试以写入模式打开文件，但更精确地分析错误类型
    match OpenOptions::new().write(true).create(false).open(path) {
        Ok(_) => false,
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => check_file_locking_status(path),
            ErrorKind::NotFound => false,
            _ => {
                eprintln!("警告: 文件打开失败，可能被占用: {} - {}", path.display(), e);
                true
            }
        },
    }
}

/// Unix 文件锁定检测
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

/// Windows特定：检查目录是否被锁定
#[cfg(target_os = "windows")]
fn is_directory_locked(path: &Path) -> bool {
    // 对于目录，尝试删除并重建来检测锁定
    // 这是一个更安全的方法，不会实际删除目录内容
    match std::fs::read_dir(path) {
        Ok(_) => {
            // 能够读取目录，通常没有被锁定
            false
        }
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    // 权限被拒绝，可能被锁定
                    eprintln!("警告: 目录访问被拒绝，可能被锁定: {}", path.display());
                    true
                }
                _ => {
                    // 其他错误，保守起见假设被锁定
                    eprintln!("警告: 目录读取失败: {} - {}", path.display(), e);
                    true
                }
            }
        }
    }
}

/// Windows特定：更精确地检查文件锁定状态
#[cfg(target_os = "windows")]
fn check_file_locking_status(path: &Path) -> bool {
    use std::fs::OpenOptions;

    // 尝试多种方式打开文件来区分锁定和权限问题
    let path_str = path.to_string_lossy();

    // 方法1：尝试以只读方式打开
    if OpenOptions::new().read(true).open(path).is_ok() {
        // 能够只读打开，但无法写入，很可能是锁定
        return true;
    }

    // 方法2：使用PowerShell检查文件句柄
    match std::process::Command::new("powershell")
        .args([
            "-Command",
            &format!(
                "Get-Process | Where-Object {{ $_.Modules.FileName -eq '{path_str}' }} | Measure-Object"
            ),
        ])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // 如果有进程在使用该文件，返回true
                !output_str.trim().contains("0")
            } else {
                // PowerShell命令失败，保守起见假设被锁定
                true
            }
        }
        Err(_) => {
            // PowerShell不可用，尝试最后的方法
            // 尝试重命名文件来检测锁定
            let temp_path = path.with_extension("tmp_check");
            match std::fs::rename(path, &temp_path) {
                Ok(_) => {
                    // 能够重命名，说明没有被锁定，立即改回来
                    let _ = std::fs::rename(&temp_path, path);
                    false
                }
                Err(_) => {
                    // 无法重命名，说明被锁定
                    true
                }
            }
        }
    }
}

/// 非Windows系统的空实现
#[cfg(not(target_os = "windows"))]
fn is_directory_locked(_path: &Path) -> bool {
    false
}

/// 非Windows系统的空实现
#[cfg(not(target_os = "windows"))]
fn check_file_locking_status(_path: &Path) -> bool {
    true
}

/// 查找占用指定文件的进程
pub fn find_processes_by_file(path: &Path) -> Result<Vec<u32>> {
    let mut pids = Vec::new();

    if !path.exists() {
        return Ok(pids);
    }

    if cfg!(target_os = "windows") {
        // Windows 系统的实现 - 使用多种方法查找占用进程
        let path_str = path.to_string_lossy();

        // 方法1：使用 handle.exe 工具（如果有）
        if let Ok(handle_pids) = find_processes_with_handle(&path_str) {
            pids.extend(handle_pids);
        }

        // 方法2：使用 PowerShell 查找占用文件的进程（更全面的方法）
        if let Ok(ps_pids) = find_processes_with_powershell(&path_str) {
            for pid in ps_pids {
                if !pids.contains(&pid) {
                    pids.push(pid);
                }
            }
        }

        // 方法3：使用 wmic 命令查找
        if let Ok(wmic_pids) = find_processes_with_wmic(&path_str) {
            for pid in wmic_pids {
                if !pids.contains(&pid) {
                    pids.push(pid);
                }
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

/// Windows特定：使用 handle.exe 查找占用进程
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
                    // handle.exe 输出格式通常是：pid: process_name path
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
            // handle.exe 不可用，这是正常的
        }
    }

    Ok(pids)
}

/// Windows特定：使用 PowerShell 查找占用进程
#[cfg(target_os = "windows")]
fn find_processes_with_powershell(path_str: &str) -> Result<Vec<u32>> {
    let mut pids = Vec::new();

    // 使用更全面的PowerShell命令
    let powershell_commands = vec![
        // 方法1：查找进程模块
        format!(
            "Get-Process | Where-Object {{$_.MainModule.FileName -like '*{}*'}} | Select-Object -ExpandProperty Id",
            path_str
        ),
        // 方法2：查找进程句柄
        format!(
            "$path = '{}'; Get-Process | ForEach-Object {{ if ($_.Modules.FileName -contains $path) {{ $_.Id }} }}",
            path_str
        ),
        // 方法3：使用 wmic 通过 PowerShell
        format!(
            "wmic process where 'ExecutablePath like \"%{}%\"' get ProcessId /format:list",
            path_str
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
                // PowerShell 命令失败，继续尝试下一个
                continue;
            }
        }
    }

    Ok(pids)
}

/// Windows特定：使用 wmic 查找占用进程
#[cfg(target_os = "windows")]
fn find_processes_with_wmic(path_str: &str) -> Result<Vec<u32>> {
    let mut pids = Vec::new();

    match std::process::Command::new("wmic")
        .args([
            "process",
            "where",
            &format!("ExecutablePath like '%{path_str}%'"),
            "get",
            "ProcessId",
            "/format:list",
        ])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                let output_str = safe_command_output_to_string(&output.stdout);
                for line in output_str.lines() {
                    if line.starts_with("ProcessId=") {
                        let pid_part = line.trim_start_matches("ProcessId=");
                        if let Ok(pid) = pid_part.trim().parse::<u32>() {
                            pids.push(pid);
                        }
                    }
                }
            }
        }
        Err(_) => {
            // wmic 命令失败
        }
    }

    Ok(pids)
}

/// 非Windows系统的空实现
#[cfg(not(target_os = "windows"))]
fn find_processes_with_handle(_path_str: &str) -> Result<Vec<u32>> {
    Ok(vec![])
}

/// 非Windows系统的空实现
#[cfg(not(target_os = "windows"))]
fn find_processes_with_powershell(_path_str: &str) -> Result<Vec<u32>> {
    Ok(vec![])
}

/// 非Windows系统的空实现
#[cfg(not(target_os = "windows"))]
fn find_processes_with_wmic(_path_str: &str) -> Result<Vec<u32>> {
    Ok(vec![])
}
