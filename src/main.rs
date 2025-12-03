mod cli;
mod port;
mod process;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    if let Err(e) = run() {
        ui::display_error(&e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Find { port } => handle_find(port)?,
        Commands::Kill { ports } => handle_kill(ports)?,
        Commands::List => handle_list()?,
    }
    
    Ok(())
}

fn handle_find(port: u16) -> Result<()> {
    match port::find_process_by_port(port) {
        Ok(process) => {
            ui::display_process_info(port, &process);
        }
        Err(_) => {
            ui::display_port_not_found(port);
        }
    }
    Ok(())
}

fn handle_kill(ports: Vec<u16>) -> Result<()> {
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
    
    // 交互式选择要终止的进程
    let selected = ui::select_processes_to_kill(port_infos)?;
    
    if selected.is_empty() {
        return Ok(());
    }
    
    // 终止选中的进程
    let pids: Vec<u32> = selected.iter().map(|info| info.process.pid).collect();
    let results = process::kill_processes(&pids);
    
    // 显示结果
    ui::display_kill_results(&results);
    
    Ok(())
}

fn handle_list() -> Result<()> {
    let port_infos = port::list_all_ports()?;
    ui::display_port_list(port_infos);
    Ok(())
}
