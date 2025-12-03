use anyhow::{anyhow, Result};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

/// 终止指定 PID 的进程
pub fn kill_process(pid: u32) -> Result<()> {
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything())
    );
    sys.refresh_all();
    
    let pid_obj = sysinfo::Pid::from_u32(pid);
    
    if let Some(process) = sys.process(pid_obj) {
        if process.kill() {
            Ok(())
        } else {
            Err(anyhow!("无法终止进程 {} (可能需要管理员权限)", pid))
        }
    } else {
        Err(anyhow!("进程 {} 不存在", pid))
    }
}

/// 批量终止进程
pub fn kill_processes(pids: &[u32]) -> Vec<(u32, Result<()>)> {
    pids.iter()
        .map(|&pid| (pid, kill_process(pid)))
        .collect()
}

