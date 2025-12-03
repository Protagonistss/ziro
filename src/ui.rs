use crate::port::PortInfo;
use anyhow::Result;
use colored::*;
use inquire::{MultiSelect, Confirm};

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

/// 树形结构展示多个端口信息
pub fn display_ports_tree(ports: &[u16], port_infos: Vec<PortInfo>) {
    if ports.is_empty() {
        return;
    }
    
    println!("{}", "⚡ 端口查询结果".cyan().bold());
    println!();
    
    // 创建端口到进程信息的映射
    let mut port_map = std::collections::HashMap::new();
    for info in port_infos {
        port_map.insert(info.port, info);
    }
    
    let total = ports.len();
    for (index, &port) in ports.iter().enumerate() {
        let is_last = index == total - 1;
        let branch = if is_last { "└─" } else { "├─" };
        let continuation = if is_last { "   " } else { "│  " };
        
        if let Some(info) = port_map.get(&port) {
            // 端口被占用
            println!("{} {} {}", branch, format!("{}", port).yellow().bold(), "✓".green());
            
            // 进程信息
            println!("{}├─ {}: {} ({})", 
                continuation,
                "进程".cyan(),
                info.process.name.green(),
                format!("{}", info.process.pid).bright_black()
            );
            
            // 命令
            let cmd = truncate_string(&info.process.cmd.join(" "), 60);
            println!("{}├─ {}: {}", 
                continuation,
                "命令".cyan(),
                cmd.bright_black()
            );
            
            // 资源使用
            println!("{}└─ {}: {} CPU, {} 内存",
                continuation,
                "资源".cyan(),
                format!("{:.1}%", info.process.cpu_usage).magenta(),
                format!("{} MB", info.process.memory / 1024 / 1024).magenta()
            );
        } else {
            // 端口空闲
            println!("{} {} {} {}", 
                branch,
                format!("{}", port).yellow().bold(),
                "✗".red(),
                "(空闲)".bright_black()
            );
        }
        
        if !is_last {
            println!("{}", continuation);
        }
    }
}

/// 树形结构展示所有端口占用情况（用于 list 命令）
pub fn display_ports_tree_all(port_infos: Vec<PortInfo>) {
    if port_infos.is_empty() {
        println!("{}", "当前没有端口被占用".yellow());
        return;
    }
    
    println!("{} {}", 
        "⚡ 端口占用情况".cyan().bold(),
        format!("(共 {} 个)", port_infos.len()).bright_black()
    );
    println!();
    
    let total = port_infos.len();
    for (index, info) in port_infos.iter().enumerate() {
        let is_last = index == total - 1;
        let branch = if is_last { "└─" } else { "├─" };
        let continuation = if is_last { "   " } else { "│  " };
        
        // 端口号和状态
        println!("{} {} {}", branch, format!("{}", info.port).yellow().bold(), "✓".green());
        
        // 进程信息
        println!("{}├─ {}: {} ({})", 
            continuation,
            "进程".cyan(),
            info.process.name.green(),
            format!("{}", info.process.pid).bright_black()
        );
        
        // 命令
        let cmd = truncate_string(&info.process.cmd.join(" "), 60);
        println!("{}├─ {}: {}", 
            continuation,
            "命令".cyan(),
            cmd.bright_black()
        );
        
        // 资源使用
        println!("{}└─ {}: {} CPU, {} 内存",
            continuation,
            "资源".cyan(),
            format!("{:.1}%", info.process.cpu_usage).magenta(),
            format!("{} MB", info.process.memory / 1024 / 1024).magenta()
        );
        
        if !is_last {
            println!("{}", continuation);
        }
    }
}

