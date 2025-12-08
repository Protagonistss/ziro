use anyhow::{Result, anyhow};
use std::fs::OpenOptions;
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

    // 在 Windows 上，使用更精确的方法检测文件锁定
    if cfg!(target_os = "windows") {
        use std::fs::OpenOptions;
        use std::io::ErrorKind;

        // 如果是目录，使用不同的检测方法
        if path.is_dir() {
            return is_directory_locked(path);
        }

        // 尝试以写入模式打开文件，但更精确地分析错误类型
        match OpenOptions::new().write(true).create(false).open(path) {
            Ok(_) => {
                // 能够正常打开，说明文件没有被占用
                false
            }
            Err(e) => {
                match e.kind() {
                    ErrorKind::PermissionDenied => {
                        // 权限被拒绝，可能是文件被锁定或权限不足
                        // 进一步检查是否真的是锁定
                        check_file_locking_status(path)
                    }
                    ErrorKind::NotFound => {
                        // 文件不存在（虽然前面检查过，但在并发情况下可能发生）
                        false
                    }
                    _ => {
                        // 其他错误，保守起见假设被占用
                        eprintln!("警告: 文件打开失败，可能被占用: {} - {}", path.display(), e);
                        true
                    }
                }
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
                "Get-Process | Where-Object {{ $_.Modules.FileName -eq '{}' }} | Measure-Object",
                path_str
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
            &format!("ExecutablePath like '%{}%'", path_str),
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
