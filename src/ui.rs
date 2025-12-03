use crate::port::{PortInfo, ProcessInfo};
use anyhow::Result;
use colored::*;
use inquire::{MultiSelect, Confirm};
use tabled::{Table, Tabled, settings::Style};

/// 显示单个进程信息
pub fn display_process_info(port: u16, process: &ProcessInfo) {
    println!("{}", "找到占用端口的进程：".green().bold());
    println!("  {}: {}", "端口".cyan(), port);
    println!("  {}: {}", "PID".cyan(), process.pid);
    println!("  {}: {}", "名称".cyan(), process.name);
    println!("  {}: {}", "命令".cyan(), process.cmd.join(" "));
    println!("  {}: {:.1}%", "CPU".cyan(), process.cpu_usage);
    println!("  {}: {} MB", "内存".cyan(), process.memory / 1024 / 1024);
}

/// 显示端口未被占用的消息
pub fn display_port_not_found(port: u16) {
    println!("{}", format!("端口 {} 未被占用", port).yellow());
}

/// 显示多个端口信息（交互式选择）
pub fn select_processes_to_kill(port_infos: Vec<PortInfo>) -> Result<Vec<PortInfo>> {
    if port_infos.is_empty() {
        println!("{}", "未找到任何占用指定端口的进程".yellow());
        return Ok(vec![]);
    }
    
    let options: Vec<String> = port_infos
        .iter()
        .map(|info| {
            format!(
                "端口 {} - {} (PID: {}) - {}",
                info.port,
                info.process.name,
                info.process.pid,
                info.process.cmd.join(" ")
            )
        })
        .collect();
    
    // 默认全选（使用索引数组）
    let defaults: Vec<usize> = (0..options.len()).collect();
    
    let selected = MultiSelect::new("选择要终止的进程：", options)
        .with_default(&defaults)
        .prompt()?;
    
    // 找出被选中的进程
    let mut result = Vec::new();
    for selection in selected {
        for info in &port_infos {
            let expected = format!(
                "端口 {} - {} (PID: {}) - {}",
                info.port,
                info.process.name,
                info.process.pid,
                info.process.cmd.join(" ")
            );
            if selection == expected {
                result.push(info.clone());
                break;
            }
        }
    }
    
    if result.is_empty() {
        println!("{}", "未选择任何进程".yellow());
        return Ok(vec![]);
    }
    
    // 确认操作
    let confirm = Confirm::new("确认终止这些进程？")
        .with_default(false)
        .prompt()?;
    
    if confirm {
        Ok(result)
    } else {
        println!("{}", "操作已取消".yellow());
        Ok(vec![])
    }
}

/// 显示终止结果
pub fn display_kill_results(results: &[(u32, Result<()>)]) {
    for (pid, result) in results {
        match result {
            Ok(()) => println!("{} {}", "✓".green(), format!("成功终止进程 {}", pid).green()),
            Err(e) => println!("{} {}: {}", "✗".red(), format!("无法终止进程 {}", pid).red(), e),
        }
    }
}

/// 显示端口列表的表格
#[derive(Tabled)]
struct PortTableRow {
    #[tabled(rename = "端口")]
    port: u16,
    #[tabled(rename = "PID")]
    pid: u32,
    #[tabled(rename = "名称")]
    name: String,
    #[tabled(rename = "命令")]
    cmd: String,
    #[tabled(rename = "CPU")]
    cpu: String,
    #[tabled(rename = "内存")]
    memory: String,
}

pub fn display_port_list(port_infos: Vec<PortInfo>) {
    if port_infos.is_empty() {
        println!("{}", "当前没有端口被占用".yellow());
        return;
    }
    
    println!("{}", "当前端口占用情况：".green().bold());
    
    let rows: Vec<PortTableRow> = port_infos
        .iter()
        .map(|info| PortTableRow {
            port: info.port,
            pid: info.process.pid,
            name: info.process.name.clone(),
            cmd: truncate_string(&info.process.cmd.join(" "), 40),
            cpu: format!("{:.1}%", info.process.cpu_usage),
            memory: format!("{} MB", info.process.memory / 1024 / 1024),
        })
        .collect();
    
    let table = Table::new(rows).with(Style::rounded()).to_string();
    println!("{}", table);
}

/// 截断字符串到指定长度
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// 显示错误信息
pub fn display_error(error: &anyhow::Error) {
    eprintln!("{} {}", "错误:".red().bold(), error);
}

