use crate::core::fs_ops::FileInfo;
use crate::core::port::PortInfo;
use crate::core::top::ProcessView;
use crate::ui::Theme;
use anyhow::Result;
use console::{Alignment, pad_str};
use inquire::{Confirm, MultiSelect};
use std::io::{self, Write};

/// 显示端口未被占用的消息
pub fn display_port_not_found(port: u16) {
    let theme = Theme::new();
    println!("{}", theme.warn(format!("端口 {port} 未被占用")));
}

/// 显示多个端口信息（交互式选择）
pub fn select_processes_to_kill(port_infos: Vec<PortInfo>) -> Result<Vec<PortInfo>> {
    let theme = Theme::new();

    if port_infos.is_empty() {
        println!("{}", theme.warn("未找到任何占用指定端口的进程"));
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
        println!("{}", theme.warn("未选择任何进程"));
        return Ok(vec![]);
    }

    // 确认操作
    let confirm = Confirm::new("确认终止这些进程？")
        .with_default(false)
        .prompt()?;

    if confirm {
        Ok(result)
    } else {
        println!("{}", theme.warn("操作已取消"));
        Ok(vec![])
    }
}

/// 显示终止结果
pub fn display_kill_results(results: &[(u32, Result<()>)]) {
    let theme = Theme::new();

    for (pid, result) in results {
        match result {
            Ok(()) => println!(
                "{} {}",
                theme.icon_success(),
                theme.success(format!("成功终止进程 {pid}"))
            ),
            Err(e) => println!(
                "{} {}: {}",
                theme.icon_error(),
                theme.error(format!("无法终止进程 {pid}")),
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
    let theme = Theme::new();
    eprintln!("{} {}", theme.error_bold("错误:"), error);
}

/// 树形结构展示多个端口信息
pub fn display_ports_tree(ports: &[u16], port_infos: Vec<PortInfo>) {
    if ports.is_empty() {
        return;
    }

    let theme = Theme::new();

    println!("{} {}", theme.icon_lightning(), theme.title("端口查询结果"));
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
                theme.highlight(port.to_string()),
                theme.icon_success()
            );

            // 进程信息
            println!(
                "{}├─ {}: {} ({})",
                continuation,
                theme.info("进程"),
                theme.success(&info.process.name),
                theme.muted(info.process.pid.to_string())
            );

            // 命令
            let cmd = truncate_string(&info.process.cmd.join(" "), 60);
            println!(
                "{}├─ {}: {}",
                continuation,
                theme.info("命令"),
                theme.muted(cmd)
            );

            // 资源使用
            println!(
                "{}└─ {}: {} CPU, {} 内存",
                continuation,
                theme.info("资源"),
                theme.accent(format!("{:.1}%", info.process.cpu_usage)),
                theme.accent(format!("{} MB", info.process.memory / 1024 / 1024))
            );
        } else {
            // 端口空闲
            println!(
                "{} {} {} {}",
                branch,
                theme.highlight(port.to_string()),
                theme.icon_error(),
                theme.muted("(空闲)")
            );
        }

        if !is_last {
            println!("{continuation}");
        }
    }
}

/// 树形结构展示所有端口占用情况（用于 list 命令）
pub fn display_ports_tree_all(port_infos: Vec<PortInfo>) {
    let theme = Theme::new();

    if port_infos.is_empty() {
        println!("{}", theme.warn("当前没有端口被占用"));
        return;
    }

    println!(
        "{} {} {}",
        theme.icon_lightning(),
        theme.title("端口占用情况"),
        theme.muted(format!("(共 {} 个)", port_infos.len()))
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
            theme.highlight(info.port.to_string()),
            theme.icon_success()
        );

        // 进程信息
        println!(
            "{}├─ {}: {} ({})",
            continuation,
            theme.info("进程"),
            theme.success(&info.process.name),
            theme.muted(info.process.pid.to_string())
        );

        // 命令
        let cmd = truncate_string(&info.process.cmd.join(" "), 60);
        println!(
            "{}├─ {}: {}",
            continuation,
            theme.info("命令"),
            theme.muted(cmd)
        );

        // 资源使用
        println!(
            "{}└─ {}: {} CPU, {} 内存",
            continuation,
            theme.info("资源"),
            theme.accent(format!("{:.1}%", info.process.cpu_usage)),
            theme.accent(format!("{} MB", info.process.memory / 1024 / 1024))
        );

        if !is_last {
            println!("{continuation}");
        }
    }
}

/// 显示删除预览
pub fn display_deletion_preview(files: &[FileInfo]) {
    let theme = Theme::new();
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
        theme.title("统计:"),
        theme.success(format!("{file_count} 个文件")),
        theme.blue(format!("{dir_count} 个目录")),
        theme.warn(format!(
            "总大小: {}",
            crate::core::fs_ops::format_size(total_size)
        ))
    );
    println!();

    // 显示文件列表预览
    let total = files.len().min(10); // 最多显示10个项目
    for file in files.iter().take(total) {
        let icon = if file.is_dir {
            theme.icon_folder()
        } else if file.is_symlink {
            theme.icon_link()
        } else {
            theme.icon_file()
        };

        let size_str = if !file.is_dir && !file.is_symlink {
            let size = format!(" ({})", crate::core::fs_ops::format_size(file.size));
            theme.muted(size)
        } else {
            String::new()
        };

        let file_type = if file.is_dir {
            theme.blue("目录")
        } else if file.is_symlink {
            theme.accent("符号链接")
        } else {
            theme.success("文件")
        };

        println!(
            "  {} {} {}{}",
            icon,
            file.path.display(),
            file_type,
            size_str
        );
    }

    if files.len() > 10 {
        println!(
            "{}",
            theme.muted(format!("  ... 还有 {} 个项目", files.len() - 10))
        );
    }

    println!();
}

/// 确认删除操作
pub fn confirm_deletion(files: &[FileInfo], force: bool, dry_run: bool) -> Result<bool> {
    let theme = Theme::new();

    if dry_run {
        println!(
            "{} {}",
            theme.icon_search(),
            theme.info_bold("预览模式 - 不会实际删除文件")
        );
        display_deletion_preview(files);
        return Ok(true);
    }

    if force {
        return Ok(true);
    }

    println!(
        "{} {}",
        theme.icon_warning(),
        theme.error_bold("即将删除以下内容")
    );
    display_deletion_preview(files);

    let confirm = Confirm::new("确认删除这些内容？此操作不可撤销！")
        .with_default(false)
        .with_help_message("使用 --force 参数可以跳过此确认")
        .prompt()?;

    Ok(confirm)
}

/// 显示删除结果
pub fn display_removal_results(
    results: &[(std::path::PathBuf, Result<()>)],
    dry_run: bool,
    verbose: bool,
) {
    let theme = Theme::new();
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

    // 如果不是 verbose 模式，只显示汇总信息
    if !verbose {
        println!(
            "{} {} {}",
            theme.title("操作完成"),
            theme.success(format!("成功: {success_count}")),
            theme.error(format!("失败: {error_count}"))
        );

        // 只有在错误模式下才显示失败的文件
        if error_count > 0 {
            for (path, result) in results {
                if let Err(e) = result {
                    println!(
                        "{} {} {}",
                        theme.icon_error(),
                        theme.error(format!("无法删除 {}", path.display())),
                        e
                    );
                }
            }
        }
        return;
    }

    // Verbose 模式：显示所有详细信息
    println!(
        "{} {} {}",
        theme.title("操作完成"),
        theme.success(format!("成功: {success_count}")),
        theme.error(format!("失败: {error_count}"))
    );

    for (path, result) in results {
        match result {
            Ok(()) => println!(
                "{} {}",
                theme.icon_success(),
                theme.muted(format!("{} {}", action, path.display()))
            ),
            Err(e) => println!(
                "{} {} {}",
                theme.icon_error(),
                theme.error(format!("无法删除 {}", path.display())),
                e
            ),
        }
    }
}

/// 显示强制终止结果
pub fn display_kill_results_force(port_infos: &[PortInfo], results: &[(u32, Result<()>)]) {
    let theme = Theme::new();

    println!("{} {}", theme.icon_fire(), theme.error_bold("强制终止进程"));
    println!();

    // 首先显示要终止的进程信息
    println!("{}", theme.title("目标进程:"));
    for info in port_infos {
        println!(
            "  端口 {} - {} (PID: {})",
            theme.highlight(info.port.to_string()),
            theme.success(&info.process.name),
            theme.muted(info.process.pid.to_string())
        );
    }
    println!();

    // 显示终止结果
    println!("{}", theme.title("终止结果:"));
    let mut success_count = 0;
    let mut error_count = 0;

    for (pid, result) in results {
        match result {
            Ok(()) => {
                success_count += 1;
                println!(
                    "{} {}",
                    theme.icon_success(),
                    theme.success(format!("成功强制终止进程 {pid}"))
                );
            }
            Err(e) => {
                error_count += 1;
                println!(
                    "{} {}: {}",
                    theme.icon_error(),
                    theme.error(format!("无法强制终止进程 {pid}")),
                    e
                );
            }
        }
    }

    println!();
    println!(
        "{} {} {}",
        theme.title("强制终止完成"),
        theme.success(format!("成功: {success_count}")),
        theme.error(format!("失败: {error_count}"))
    );
}

/// 实时进程内存展示
pub struct TopRenderOptions {
    pub total_memory: u64,
    pub used_memory: u64,
    pub refresh: u64,
    pub interval: f32,
    pub show_cpu: bool,
    pub show_cmd: bool,
    pub incremental: bool,
}

pub fn display_top(
    processes: &[ProcessView],
    opts: TopRenderOptions,
    last_frame: &mut Vec<String>,
) {
    let theme = Theme::new();

    // 列宽配置（使用 console::pad_str，支持中日韩宽字符）
    const RANK_W: usize = 4;
    const NAME_W: usize = 26;
    const PID_W: usize = 10;
    const MEM_W: usize = 10;
    const MEM_PCT_W: usize = 7;
    const CPU_W: usize = 8;

    let mut lines: Vec<String> = Vec::new();

    // 顶部摘要，添加实时状态指示
    let status_icon = if opts.refresh.is_multiple_of(2) {
        "●"
    } else {
        "◐"
    };
    lines.push(format!(
        "{} {} {}",
        theme.icon_lightning(),
        theme.title("进程内存占用"),
        theme.muted(format!("[{status_icon}]"))
    ));

    let mem_used_str = crate::core::fs_ops::format_size(opts.used_memory);
    let mem_total_str = crate::core::fs_ops::format_size(opts.total_memory);
    let mem_pct = if opts.total_memory > 0 {
        (opts.used_memory as f64 / opts.total_memory as f64) * 100.0
    } else {
        0.0
    };

    // 创建实时状态行
    let status_line = format!(
        "刷新: {} | 间隔: {:.1}s | 进程: {} | 内存: {} / {} ({:.1}%) | {}",
        opts.refresh,
        opts.interval,
        processes.len(),
        mem_used_str,
        mem_total_str,
        mem_pct,
        theme.muted("Ctrl+C 退出")
    );
    lines.push(status_line);

    // 添加进度条显示内存使用率
    let bar_width = 30;
    let filled = (mem_pct / 100.0 * bar_width as f64).round() as usize;
    let bar = "=".repeat(filled) + &"·".repeat(bar_width - filled);
    lines.push(theme.muted(format!("[{bar}]")).to_string());
    lines.push(String::new());

    let header_rank = pad_str("序号", RANK_W, Alignment::Left, None);
    let header_name = pad_str("名称", NAME_W, Alignment::Left, None);
    let header_pid = pad_str("PID", PID_W, Alignment::Left, None);
    let header_mem = pad_str("内存", MEM_W, Alignment::Right, None);
    let header_mem_pct = pad_str("Mem%", MEM_PCT_W, Alignment::Right, None);
    let header_cpu = pad_str("CPU", CPU_W, Alignment::Right, None);
    let header_cmd = if opts.show_cmd { "命令" } else { "" };

    lines.push(format!(
        "{header_rank} {header_name} {header_pid} {header_mem} {header_mem_pct} {header_cpu} {header_cmd}"
    ));

    let sep_len = RANK_W + NAME_W + PID_W + MEM_W + MEM_PCT_W + CPU_W + 6; // spaces between columns
    lines.push(theme.muted("-".repeat(sep_len)).to_string());

    for (index, process) in processes.iter().enumerate() {
        let rank = index + 1;
        let rank_plain = rank.to_string();
        let rank_colored = match rank {
            1 => theme.highlight(&rank_plain),
            2 => theme.warn(&rank_plain),
            3 => theme.info(&rank_plain),
            _ => theme.muted(&rank_plain),
        };

        let mem_str = crate::core::fs_ops::format_size(process.memory_bytes);
        let mem_pct_str = format!("{:.1}%", process.memory_percent);
        let cpu_str = if opts.show_cpu {
            format!("{:.1}%", process.cpu)
        } else {
            "-".to_string()
        };

        let name_plain = truncate_string(&process.name, NAME_W.saturating_sub(2));
        let pid_plain = process.pid.to_string();
        let cmd_display = if opts.show_cmd && !process.cmd.is_empty() {
            format!(" {}", theme.muted(truncate_string(&process.cmd, 60)))
        } else {
            String::new()
        };

        let name_padded = pad_str(&name_plain, NAME_W, Alignment::Left, None);
        let pid_padded = pad_str(&pid_plain, PID_W, Alignment::Left, None);
        let mem_padded = pad_str(&mem_str, MEM_W, Alignment::Right, None);
        let mem_pct_padded = pad_str(&mem_pct_str, MEM_PCT_W, Alignment::Right, None);
        let cpu_padded = pad_str(&cpu_str, CPU_W, Alignment::Right, None);

        let name_cell = theme.success(name_padded);
        let pid_cell = theme.muted(pid_padded);
        let mem_cell = theme.warn(mem_padded);
        let mem_pct_cell = theme.warn(mem_pct_padded);
        let cpu_cell = theme.accent(cpu_padded);

        let rank_cell = pad_str(&rank_colored, RANK_W, Alignment::Left, None);

        lines.push(format!(
            "{rank_cell} {name_cell} {pid_cell} {mem_cell} {mem_pct_cell} {cpu_cell}{cmd_display}"
        ));
    }

    render_frame(&lines, opts.incremental, last_frame);
}

/// 将构建好的行以增量方式输出到终端
fn render_frame(lines: &[String], incremental: bool, last_frame: &mut Vec<String>) {
    if !incremental {
        for line in lines {
            println!("{line}");
        }
        return;
    }

    let mut stdout = io::stdout();

    // 隐藏光标，避免闪烁
    let _ = write!(stdout, "\x1b[?25l");

    // 回到屏幕左上角，使用更高效的控制序列
    let _ = write!(stdout, "\x1b[H");

    let max_len = lines.len().max(last_frame.len());
    let mut changed_lines = 0;

    for i in 0..max_len {
        match (lines.get(i), last_frame.get(i)) {
            (Some(new_line), Some(old_line)) if new_line == old_line => {
                // 内容一致，快速移到下一行
                let _ = write!(stdout, "\x1b[E");
            }
            (Some(new_line), _) => {
                // 新行或内容变化，清理并输出
                let _ = write!(stdout, "\x1b[2K{new_line}\r\n");
                changed_lines += 1;
            }
            (None, Some(_)) => {
                // 旧行需要清理
                let _ = write!(stdout, "\x1b[2K\r\n");
                changed_lines += 1;
            }
            (None, None) => break,
        }
    }

    // 只清理多余的行，减少不必要的操作
    if max_len > lines.len() {
        let _ = write!(stdout, "\x1b[{}J", max_len - lines.len() + 1);
    } else {
        // 清理光标到屏幕末尾
        let _ = write!(stdout, "\x1b[J");
    }

    // 显示光标
    let _ = write!(stdout, "\x1b[?25h");

    // 立即刷新输出缓冲区
    let _ = stdout.flush();

    // 高效更新 last_frame
    if changed_lines > 0 || lines.len() != last_frame.len() {
        last_frame.clear();
        last_frame.extend(lines.iter().cloned());
    }
}
