use anyhow::Result;
use std::collections::HashMap;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

/// Process information
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmd: Vec<String>,
    pub cpu_usage: f32,
    pub memory: u64,
}

impl ProcessInfo {
    /// Create ProcessInfo from a sysinfo::Process
    fn from_sysinfo(pid: u32, process: &sysinfo::Process) -> Self {
        ProcessInfo {
            pid,
            name: process.name().to_string_lossy().to_string(),
            cmd: process
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect(),
            cpu_usage: process.cpu_usage(),
            memory: process.memory(),
        }
    }
}

/// Port usage information
#[derive(Debug, Clone)]
pub struct PortInfo {
    pub port: u16,
    pub process: ProcessInfo,
}

/// Find processes occupying multiple ports
pub fn find_processes_by_ports(ports: &[u16]) -> Result<Vec<PortInfo>> {
    let connections = get_network_connections()?;
    let sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );

    let mut result = Vec::new();

    for &port in ports {
        if let Some(&pid) = connections.get(&port)
            && let Some(process) = sys.process(sysinfo::Pid::from_u32(pid))
        {
            let process_info = ProcessInfo::from_sysinfo(pid, process);
            result.push(PortInfo {
                port,
                process: process_info,
            });
        }
    }

    Ok(result)
}

/// List all port usage
pub fn list_all_ports() -> Result<Vec<PortInfo>> {
    let connections = get_network_connections()?;
    let sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );

    let mut result = Vec::new();

    for (port, pid) in connections {
        if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
            let process_info = ProcessInfo::from_sysinfo(pid, process);
            result.push(PortInfo {
                port,
                process: process_info,
            });
        }
    }

    // Sort by port number
    result.sort_by_key(|info| info.port);

    Ok(result)
}

/// Get network connection information (port -> PID mapping)
#[cfg(target_os = "windows")]
fn get_network_connections() -> Result<HashMap<u16, u32>> {
    use std::process::Command;

    let output = Command::new("netstat").args(["-ano"]).output()?;

    // Use simple string processing to avoid encoding conversion issues
    let connections = parse_netstat_output(&output.stdout)?;

    Ok(connections)
}

/// Parse netstat output, extract port-to-PID mapping
#[cfg(target_os = "windows")]
fn parse_netstat_output(stdout: &[u8]) -> Result<HashMap<u16, u32>> {
    let mut connections = HashMap::new();

    // Use lossy conversion directly to avoid complex encoding detection
    let text = String::from_utf8_lossy(stdout);

    for line in text.lines() {
        // Skip header lines and empty lines
        if line.trim().is_empty()
            || line.contains("Active")
            || line.contains("Proto")
            || line.contains("协议")
        {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            // Use the last number as PID
            if let Some(pid_str) = parts.last() {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    // Extract local address and port (usually the second element)
                    if let Some(local_addr) = parts.get(1) {
                        if let Some(port_str) = local_addr.rsplit(':').next() {
                            if let Ok(port) = port_str.parse::<u16>() {
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

    // Read TCP connections
    for path in &["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(content) = fs::read_to_string(path) {
            parse_proc_net(&content, &mut connections)?;
        }
    }

    // Read UDP connections
    for path in &["/proc/net/udp", "/proc/net/udp6"] {
        if let Ok(content) = fs::read_to_string(path) {
            parse_proc_net(&content, &mut connections)?;
        }
    }

    Ok(connections)
}

#[cfg(target_os = "linux")]
fn parse_proc_net(content: &str, connections: &mut HashMap<u16, u32>) -> Result<()> {
    for (port, inode) in parse_proc_net_entries(content) {
        if let Ok(pid) = find_pid_by_inode(inode) {
            connections.insert(port, pid);
        }
    }
    Ok(())
}

/// Parse /proc/net/tcp entries, returning (port, inode) pairs.
#[cfg(any(target_os = "linux", test))]
fn parse_proc_net_entries(content: &str) -> Vec<(u16, u64)> {
    let mut entries = Vec::new();
    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 10 {
            if let Some(local_addr) = parts.get(1) {
                if let Some(port_hex) = local_addr.split(':').nth(1) {
                    if let Ok(port) = u16::from_str_radix(port_hex, 16) {
                        if let Some(inode_str) = parts.get(9) {
                            if let Ok(inode) = inode_str.parse::<u64>() {
                                entries.push((port, inode));
                            }
                        }
                    }
                }
            }
        }
    }
    entries
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
        "No PID found for inode {inode}"
    )))
}

#[cfg(target_os = "macos")]
fn get_network_connections() -> Result<HashMap<u16, u32>> {
    use std::process::Command;

    let output = Command::new("lsof").args(["-i", "-n", "-P"]).output()?;

    // Use simple string processing to avoid encoding conversion issues
    let connections = parse_lsof_output(&output.stdout)?;

    Ok(connections)
}

/// Parse lsof output, extract port-to-PID mapping
#[cfg(any(target_os = "macos", test))]
fn parse_lsof_output(stdout: &[u8]) -> Result<HashMap<u16, u32>> {
    let mut connections = HashMap::new();

    // Use lossy conversion directly to avoid complex encoding detection
    let text = String::from_utf8_lossy(stdout);

    for line in text.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 9 {
            // COMMAND   PID   USER   FD   TYPE   DEVICE SIZE/OFF NODE NAME
            // node    12345   user   21u  IPv4   0x...      0t0  TCP *:8080 (LISTEN)
            if let Ok(pid) = parts[1].parse::<u32>() {
                if let Some(name) = parts.get(8) {
                    // Parse port (format: *:8080 or 127.0.0.1:8080)
                    if let Some(port_str) = name.rsplit(':').next() {
                        // Remove possible status info such as (LISTEN)
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
    Err(anyhow::Error::msg(
        "Network connection queries are not supported on the current operating system",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn test_parse_netstat_output() {
        let input = b"\
Active Connections

  Proto  Local Address          Foreign Address        State           PID
  TCP    0.0.0.0:8080           0.0.0.0:0              LISTENING       1234
  TCP    0.0.0.0:443            0.0.0.0:0              LISTENING       5678
  TCP    [::]:3000              [::]:0                 LISTENING       9012
";
        let result = parse_netstat_output(input).unwrap();
        assert_eq!(result.get(&8080), Some(&1234));
        assert_eq!(result.get(&443), Some(&5678));
        assert_eq!(result.get(&3000), Some(&9012));
        assert_eq!(result.len(), 3);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_parse_netstat_skips_headers() {
        let input = b"\
Active Connections
  Proto  Local Address          Foreign Address        State           PID
";
        let result = parse_netstat_output(input).unwrap();
        assert!(result.is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_parse_netstat_empty() {
        let result = parse_netstat_output(b"").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_proc_net_entries() {
        let input = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 12345 1 0000000000000000 100 0 0 10 0
   1: 00000000:0016 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 67890 1 0000000000000000 100 0 0 10 0";
        let entries = parse_proc_net_entries(input);
        // 0x1F90 = 8080, 0x0016 = 22
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], (8080, 12345));
        assert_eq!(entries[1], (22, 67890));
    }

    #[test]
    fn test_parse_proc_net_entries_empty() {
        let entries = parse_proc_net_entries("  sl  local_address rem_address\n");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_lsof_output() {
        let input = b"COMMAND   PID   USER   FD   TYPE   DEVICE SIZE/OFF NODE NAME
node    12345   user   21u  IPv4 0x12345 0t0 TCP *:8080 (LISTEN)
python  67890   user   22u  IPv6 0xabcde 0t0 TCP 127.0.0.1:3000 (LISTEN)
";
        let result = parse_lsof_output(input).unwrap();
        assert_eq!(result.get(&8080), Some(&12345));
        assert_eq!(result.get(&3000), Some(&67890));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_lsof_empty() {
        let result =
            parse_lsof_output(b"COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME\n").unwrap();
        assert!(result.is_empty());
    }
}
