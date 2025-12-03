use anyhow::Result;
use std::collections::HashMap;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

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
        if let Some(&pid) = connections.get(&port) {
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut connections = HashMap::new();

    for line in stdout.lines().skip(4) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            // TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       1234
            // TCP    [::]:135               [::]:0                 LISTENING       1234
            if let Some(local_addr) = parts.get(1) {
                if let Some(port_str) = local_addr.rsplit(':').next() {
                    if let Ok(port) = port_str.parse::<u16>() {
                        if let Some(pid_str) = parts.last() {
                            if let Ok(pid) = pid_str.parse::<u32>() {
                                connections.insert(port, pid);
                            }
                        }
                    }
                }
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
                let fd_dir = PathBuf::from(format!("/proc/{}/fd", pid));
                if let Ok(fd_entries) = fs::read_dir(fd_dir) {
                    for fd_entry in fd_entries.flatten() {
                        if let Ok(link) = fs::read_link(fd_entry.path()) {
                            if let Some(link_str) = link.to_str() {
                                if link_str.contains(&format!("socket:[{}]", inode)) {
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
        "未找到 inode {} 对应的 PID",
        inode
    )))
}

#[cfg(target_os = "macos")]
fn get_network_connections() -> Result<HashMap<u16, u32>> {
    use std::process::Command;

    let output = Command::new("lsof").args(["-i", "-n", "-P"]).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
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
