mod cli;
mod file;
mod port;
mod process;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use colored::Colorize;

fn main() {
    if let Err(e) = run() {
        ui::display_error(&e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // 如果请求版本信息，显示并退出
    if cli.version {
        display_version();
        return Ok(());
    }

    match cli.command {
        Some(Commands::Find { ports }) => handle_find(ports)?,
        Some(Commands::Kill { ports, force }) => handle_kill(ports, force)?,
        Some(Commands::List) => handle_list()?,
        Some(Commands::Remove {
            paths,
            force,
            recursive,
            dry_run,
        }) => handle_remove(paths, force, recursive, dry_run)?,
        None => {
            // 当没有提供子命令时显示帮助信息
            println!("使用 'ziro --help' 查看可用命令");
        }
    }

    Ok(())
}

fn display_version() {
    let version = env!("CARGO_PKG_VERSION");
    println!("\x1b[1;36mziro\x1b[0m \x1b[1;37mv{version}\x1b[0m");
}

fn handle_find(ports: Vec<u16>) -> Result<()> {
    if ports.is_empty() {
        println!("请指定至少一个端口号");
        return Ok(());
    }

    // 批量查找所有端口
    let port_infos = port::find_processes_by_ports(&ports)?;

    // 使用树形结构展示
    ui::display_ports_tree(&ports, port_infos);

    Ok(())
}

fn handle_kill(ports: Vec<u16>, force: bool) -> Result<()> {
    if ports.is_empty() {
        println!("请指定至少一个端口号");
        return Ok(());
    }

    // 查找所有指定端口的进程
    let port_infos = port::find_processes_by_ports(&ports)?;

    if port_infos.is_empty() {
        println!("未找到占用指定端口的进程");
        for &port in &ports {
            ui::display_port_not_found(port);
        }
        return Ok(());
    }

    if force {
        // 强制模式：直接终止所有找到的进程
        let pids: Vec<u32> = port_infos.iter().map(|info| info.process.pid).collect();
        let results = process::kill_processes_force(&pids);

        // 显示强制模式的结果
        ui::display_kill_results_force(&port_infos, &results);
    } else {
        // 交互模式：让用户选择要终止的进程
        let selected = ui::select_processes_to_kill(port_infos)?;

        if selected.is_empty() {
            return Ok(());
        }

        // 终止选中的进程
        let pids: Vec<u32> = selected.iter().map(|info| info.process.pid).collect();
        let results = process::kill_processes(&pids);

        // 显示结果
        ui::display_kill_results(&results);
    }

    Ok(())
}

fn handle_list() -> Result<()> {
    let port_infos = port::list_all_ports()?;
    ui::display_ports_tree_all(port_infos);
    Ok(())
}

fn handle_remove(
    paths: Vec<std::path::PathBuf>,
    force: bool,
    recursive: bool,
    dry_run: bool,
) -> Result<()> {
    if paths.is_empty() {
        println!("请指定至少一个文件或目录路径");
        return Ok(());
    }

    // 验证路径安全性
    file::validate_paths(&paths)?;

    // 收集要删除的文件信息
    let files = file::collect_files_to_remove(&paths, recursive)?;

    if files.is_empty() {
        println!("没有找到匹配的文件或目录");
        return Ok(());
    }

    // 显示预览并确认
    if !ui::confirm_deletion(&files, force, dry_run)? {
        println!("{}", "操作已取消".bright_yellow());
        return Ok(());
    }

    // 执行删除
    let results = file::remove_files(&files, dry_run);

    // 显示结果
    ui::display_removal_results(&results, dry_run);

    Ok(())
}
