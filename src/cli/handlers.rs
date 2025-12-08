use crate::core::{fs_ops, port, process, top};
use crate::ui;
use anyhow::Result;
use colored::Colorize;
use console::Style;
use std::env;

pub fn display_version() {
    let version = env!("CARGO_PKG_VERSION");
    let cyan = Style::new().cyan().bold();
    let white = Style::new().white().bold();

    println!(
        "{} {}",
        cyan.apply_to("ziro"),
        white.apply_to(format!("v{version}"))
    );
}

pub fn handle_find(ports: Vec<u16>) -> Result<()> {
    if ports.is_empty() {
        println!("请至少指定一个端口号");
        return Ok(());
    }

    let port_infos = port::find_processes_by_ports(&ports)?;
    ui::display_ports_tree(&ports, port_infos);
    Ok(())
}

pub fn handle_kill(ports: Vec<u16>, force: bool) -> Result<()> {
    if ports.is_empty() {
        println!("请至少指定一个端口号");
        return Ok(());
    }

    let port_infos = port::find_processes_by_ports(&ports)?;

    if port_infos.is_empty() {
        println!("未找到占用指定端口的进程");
        for &port in &ports {
            ui::display_port_not_found(port);
        }
        return Ok(());
    }

    if force {
        let pids: Vec<u32> = port_infos.iter().map(|info| info.process.pid).collect();
        let results = process::kill_processes_force(&pids);
        ui::display_kill_results_force(&port_infos, &results);
    } else {
        let selected = ui::select_processes_to_kill(port_infos)?;

        if selected.is_empty() {
            return Ok(());
        }

        let pids: Vec<u32> = selected.iter().map(|info| info.process.pid).collect();
        let results = process::kill_processes(&pids);
        ui::display_kill_results(&results);
    }

    Ok(())
}

pub fn handle_list() -> Result<()> {
    let port_infos = port::list_all_ports()?;
    ui::display_ports_tree_all(port_infos);
    Ok(())
}

pub fn handle_top(interval: f32, limit: usize, cpu: bool, cmd: bool, once: bool) -> Result<()> {
    let opts = top::TopOptions {
        interval,
        limit,
        show_cpu: cpu,
        show_cmd: cmd,
        once,
    };
    top::run_top(opts)
}

pub fn handle_remove(
    paths: Vec<std::path::PathBuf>,
    force: bool,
    recursive: bool,
    dry_run: bool,
    verbose: bool,
    anyway: bool,
) -> Result<()> {
    if paths.is_empty() {
        println!("请至少指定一个文件或目录路径");
        return Ok(());
    }

    fs_ops::validate_paths(&paths)?;
    let files = fs_ops::collect_files_to_remove(&paths, recursive)?;

    if files.is_empty() {
        println!("没有匹配的文件或目录");
        return Ok(());
    }

    if !ui::confirm_deletion(&files, force, dry_run)? {
        println!("{}", "操作已取消".bright_yellow());
        return Ok(());
    }

    let results = fs_ops::remove_files(&files, dry_run, verbose, anyway);
    ui::display_removal_results(&results, dry_run, verbose);
    Ok(())
}
