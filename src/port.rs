use anyhow::Result;
use std::collections::HashMap;
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

/// 进程信息
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmd: Vec<String>,
    pub cpu_usage: f32,
    pub memory: u64,
}

/// 端口占用信息
#[derive(Debug, Clone)]
pub struct PortInfo {
    pub port: u16,
    pub process: ProcessInfo,
}

/// 查找占用多个端口的进程
pub fn find_processes_by_ports(ports: &[u16]) -> Result<Vec<PortInfo>> {
    let connections = get_network_connections()?;
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_all();

    let mut result = Vec::new();

    for &port in ports {
        if let Some(&pid) = connections.get(&port)
            && let Some(process) = sys.process(sysinfo::Pid::from_u32(pid))
        {
            let process_info = ProcessInfo {
                pid,
                name: process.name().to_string_lossy().to_string(),
                cmd: process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().to_string())
                    .collect(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
            };
            result.push(PortInfo {
                port,
                process: process_info,
            });
        }
    }

    Ok(result)
}

/// 列出所有端口占用情况
pub fn list_all_ports() -> Result<Vec<PortInfo>> {
    let connections = get_network_connections()?;
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_all();

    let mut result = Vec::new();

    for (port, pid) in connections {
        if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
            let process_info = ProcessInfo {
                pid,
                name: process.name().to_string_lossy().to_string(),
                cmd: process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().to_string())
                    .collect(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
            };
            result.push(PortInfo {
                port,
                process: process_info,
            });
        }
    }

    // 按端口号排序
    result.sort_by_key(|info| info.port);

    Ok(result)
}

/// 获取网络连接信息（端口 -> PID 映射）
#[cfg(target_os = "windows")]
fn get_network_connections() -> Result<HashMap<u16, u32>> {
    use std::process::Command;

    let output = Command::new("netstat").args(["-ano"]).output()?;

    let stdout = safe_command_output_to_string(&output.stdout);
    let mut connections = HashMap::new();

    for line in stdout.lines().skip(4) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            // TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       1234
            // TCP    [::]:135               [::]:0                 LISTENING       1234
            if let Some(local_addr) = parts.get(1)
                && let Some(port_str) = local_addr.rsplit(':').next()
                && let Ok(port) = port_str.parse::<u16>()
                && let Some(pid_str) = parts.last()
                && let Ok(pid) = pid_str.parse::<u32>()
            {
                connections.insert(port, pid);
            }
        }
    }

    Ok(connections)
}

#[cfg(target_os = "linux")]
fn get_network_connections() -> Result<HashMap<u16, u32>> {
    use std::fs;

    let mut connections = HashMap::new();

    // 读取 TCP 连接
    for path in &["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(content) = fs::read_to_string(path) {
            parse_proc_net(&content, &mut connections)?;
        }
    }

    // 读取 UDP 连接
    for path in &["/proc/net/udp", "/proc/net/udp6"] {
        if let Ok(content) = fs::read_to_string(path) {
            parse_proc_net(&content, &mut connections)?;
        }
    }

    Ok(connections)
}

#[cfg(target_os = "linux")]
fn parse_proc_net(content: &str, connections: &mut HashMap<u16, u32>) -> Result<()> {
    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 10 {
            // 解析本地地址（格式：0100007F:1F90 表示 127.0.0.1:8080）
            if let Some(local_addr) = parts.get(1) {
                if let Some(port_hex) = local_addr.split(':').nth(1) {
                    if let Ok(port) = u16::from_str_radix(port_hex, 16) {
                        // 解析 inode
                        if let Some(inode_str) = parts.get(9) {
                            if let Ok(inode) = inode_str.parse::<u64>() {
                                // 通过 inode 查找 PID
                                if let Ok(pid) = find_pid_by_inode(inode) {
                                    connections.insert(port, pid);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn find_pid_by_inode(inode: u64) -> Result<u32> {
    use std::fs;
    use std::path::PathBuf;

    let proc_dir = fs::read_dir("/proc")?;

    for entry in proc_dir.flatten() {
        if let Ok(file_name) = entry.file_name().into_string() {
            if let Ok(pid) = file_name.parse::<u32>() {
                let fd_dir = PathBuf::from(format!("/proc/{pid}/fd"));
                if let Ok(fd_entries) = fs::read_dir(fd_dir) {
                    for fd_entry in fd_entries.flatten() {
                        if let Ok(link) = fs::read_link(fd_entry.path()) {
                            if let Some(link_str) = link.to_str() {
                                if link_str.contains(&format!("socket:[{inode}]")) {
                                    return Ok(pid);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err(anyhow::Error::msg(format!(
        "未找到 inode {inode} 对应的 PID"
    )))
}

#[cfg(target_os = "macos")]
fn get_network_connections() -> Result<HashMap<u16, u32>> {
    use std::process::Command;

    let output = Command::new("lsof").args(["-i", "-n", "-P"]).output()?;

    let stdout = safe_command_output_to_string(&output.stdout);
    let mut connections = HashMap::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 9 {
            // COMMAND   PID   USER   FD   TYPE   DEVICE SIZE/OFF NODE NAME
            // node    12345   user   21u  IPv4   0x...      0t0  TCP *:8080 (LISTEN)
            if let Ok(pid) = parts[1].parse::<u32>() {
                if let Some(name) = parts.get(8) {
                    // 解析端口（格式：*:8080 或 127.0.0.1:8080）
                    if let Some(port_str) = name.rsplit(':').next() {
                        // 移除可能的状态信息，如 (LISTEN)
                        let port_str = port_str.split('(').next().unwrap_or(port_str).trim();
                        if let Ok(port) = port_str.parse::<u16>() {
                            connections.insert(port, pid);
                        }
                    }
                }
            }
        }
    }

    Ok(connections)
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn get_network_connections() -> Result<HashMap<u16, u32>> {
    Err(anyhow::Error::msg("当前操作系统不支持网络连接查询"))
}
