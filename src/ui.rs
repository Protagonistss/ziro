use crate::file::FileInfo;
use crate::port::PortInfo;
use anyhow::Result;
use colored::*;
use inquire::{Confirm, MultiSelect};

/// 显示端口未被占用的消息
pub fn display_port_not_found(port: u16) {
    println!("{}", format!("端口 {port} 未被占用").yellow());
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
            Ok(()) => println!("{} {}", "✓".green(), format!("成功终止进程 {pid}").green()),
            Err(e) => println!(
                "{} {}: {}",
                "✗".red(),
                format!("无法终止进程 {pid}").red(),
                e
            ),
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
            println!(
                "{} {} {}",
                branch,
                format!("{port}").yellow().bold(),
                "✓".green()
            );

            // 进程信息
            println!(
                "{}├─ {}: {} ({})",
                continuation,
                "进程".cyan(),
                info.process.name.green(),
                format!("{}", info.process.pid).bright_black()
            );

            // 命令
            let cmd = truncate_string(&info.process.cmd.join(" "), 60);
            println!(
                "{}├─ {}: {}",
                continuation,
                "命令".cyan(),
                cmd.bright_black()
            );

            // 资源使用
            println!(
                "{}└─ {}: {} CPU, {} 内存",
                continuation,
                "资源".cyan(),
                format!("{:.1}%", info.process.cpu_usage).magenta(),
                format!("{} MB", info.process.memory / 1024 / 1024).magenta()
            );
        } else {
            // 端口空闲
            println!(
                "{} {} {} {}",
                branch,
                format!("{port}").yellow().bold(),
                "✗".red(),
                "(空闲)".bright_black()
            );
        }

        if !is_last {
            println!("{continuation}");
        }
    }
}

/// 树形结构展示所有端口占用情况（用于 list 命令）
pub fn display_ports_tree_all(port_infos: Vec<PortInfo>) {
    if port_infos.is_empty() {
        println!("{}", "当前没有端口被占用".yellow());
        return;
    }

    println!(
        "{} {}",
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
        println!(
            "{} {} {}",
            branch,
            format!("{}", info.port).yellow().bold(),
            "✓".green()
        );

        // 进程信息
        println!(
            "{}├─ {}: {} ({})",
            continuation,
            "进程".cyan(),
            info.process.name.green(),
            format!("{}", info.process.pid).bright_black()
        );

        // 命令
        let cmd = truncate_string(&info.process.cmd.join(" "), 60);
        println!(
            "{}├─ {}: {}",
            continuation,
            "命令".cyan(),
            cmd.bright_black()
        );

        // 资源使用
        println!(
            "{}└─ {}: {} CPU, {} 内存",
            continuation,
            "资源".cyan(),
            format!("{:.1}%", info.process.cpu_usage).magenta(),
            format!("{} MB", info.process.memory / 1024 / 1024).magenta()
        );

        if !is_last {
            println!("{continuation}");
        }
    }
}

/// 显示删除预览
pub fn display_deletion_preview(files: &[FileInfo]) {
    let total_size: u64 = files.iter().map(|f| f.size).sum();
    let (file_count, dir_count) = files.iter().fold((0, 0), |(files, dirs), f| {
        if f.is_dir {
            (files, dirs + 1)
        } else {
            (files + 1, dirs)
        }
    });

    println!(
        "{} {} {} {}",
        "统计:".cyan().bold(),
        format!("{file_count} 个文件").green(),
        format!("{dir_count} 个目录").blue(),
        format!("总大小: {}", crate::file::format_size(total_size)).yellow()
    );
    println!();

    // 显示文件列表预览
    let total = files.len().min(10); // 最多显示10个项目
    for file in files.iter().take(total) {
        let icon = if file.is_dir {
            "📁".to_string()
        } else if file.is_symlink {
            "🔗".to_string()
        } else {
            "📄".to_string()
        };

        let size_str = if !file.is_dir && !file.is_symlink {
            format!(" ({})", crate::file::format_size(file.size))
        } else {
            String::new()
        };

        let file_type = if file.is_dir {
            "目录".blue()
        } else if file.is_symlink {
            "符号链接".magenta()
        } else {
            "文件".green()
        };

        println!(
            "  {} {} {}{}",
            icon,
            file.path.display(),
            file_type,
            size_str.bright_black()
        );
    }

    if files.len() > 10 {
        println!("  ... 还有 {} 个项目", files.len() - 10);
    }

    println!();
}

/// 确认删除操作
pub fn confirm_deletion(files: &[FileInfo], force: bool, dry_run: bool) -> Result<bool> {
    if dry_run {
        println!("{}", "🔍 预览模式 - 不会实际删除文件".blue().bold());
        display_deletion_preview(files);
        return Ok(true);
    }

    if force {
        return Ok(true);
    }

    println!("{}", "⚠️  即将删除以下内容".red().bold());
    display_deletion_preview(files);

    let confirm = Confirm::new("确认删除这些内容？此操作不可撤销！")
        .with_default(false)
        .with_help_message("使用 --force 参数可以跳过此确认")
        .prompt()?;

    Ok(confirm)
}

/// 显示删除结果
pub fn display_removal_results(results: &[(std::path::PathBuf, Result<()>)], dry_run: bool) {
    let action = if dry_run { "预览删除" } else { "删除" };
    let (success_count, error_count) =
        results
            .iter()
            .fold((0, 0), |(success, error), (_, result)| {
                if result.is_ok() {
                    (success + 1, error)
                } else {
                    (success, error + 1)
                }
            });

    println!(
        "{} {} {}",
        "操作完成".cyan().bold(),
        format!("成功: {success_count}").green(),
        format!("失败: {error_count}").red()
    );

    for (path, result) in results {
        match result {
            Ok(()) => println!(
                "{} {}",
                "✓".green(),
                format!("{} {}", action, path.display()).bright_black()
            ),
            Err(e) => println!(
                "{} {} {}",
                "✗".red(),
                format!("无法删除 {}", path.display()).red(),
                e
            ),
        }
    }
}

/// 显示强制终止结果
pub fn display_kill_results_force(port_infos: &[PortInfo], results: &[(u32, Result<()>)]) {
    println!("{}", "🔥 强制终止进程".red().bold());
    println!();

    // 首先显示要终止的进程信息
    println!("{}", "目标进程:".cyan().bold());
    for info in port_infos {
        println!(
            "  端口 {} - {} (PID: {})",
            info.port.to_string().yellow(),
            info.process.name.green(),
            format!("{}", info.process.pid).bright_black()
        );
    }
    println!();

    // 显示终止结果
    println!("{}", "终止结果:".cyan().bold());
    let mut success_count = 0;
    let mut error_count = 0;

    for (pid, result) in results {
        match result {
            Ok(()) => {
                success_count += 1;
                println!(
                    "{} {}",
                    "✓".green(),
                    format!("成功强制终止进程 {pid}").green()
                );
            }
            Err(e) => {
                error_count += 1;
                println!(
                    "{} {}: {}",
                    "✗".red(),
                    format!("无法强制终止进程 {pid}").red(),
                    e
                );
            }
        }
    }

    println!();
    println!(
        "{} {} {}",
        "强制终止完成".cyan().bold(),
        format!("成功: {success_count}").green(),
        format!("失败: {error_count}").red()
    );
}
