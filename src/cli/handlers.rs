use crate::core::{fs_ops, port, process, top};
use crate::ui;
use crate::ui::Theme;
use anyhow::{Result, bail};
use std::path::PathBuf;

/// Options for the remove command
pub struct RemoveOptions {
    pub paths: Vec<PathBuf>,
    pub force: bool,
    pub recursive: bool,
    pub dry_run: bool,
    pub verbose: bool,
    pub anyway: bool,
}

pub fn handle_find(ports: Vec<u16>) -> Result<()> {
    if ports.is_empty() {
        bail!("Please specify at least one port number");
    }

    let port_infos = port::find_processes_by_ports(&ports)?;
    ui::display_ports_tree(&ports, port_infos);
    Ok(())
}

pub fn handle_kill(ports: Vec<u16>, force: bool) -> Result<()> {
    if ports.is_empty() {
        bail!("Please specify at least one port number");
    }

    let port_infos = port::find_processes_by_ports(&ports)?;

    if port_infos.is_empty() {
        println!("No processes found occupying the specified ports");
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

pub fn handle_who(paths: Vec<PathBuf>) -> Result<()> {
    if paths.is_empty() {
        bail!("Please specify at least one file or directory path");
    }

    fs_ops::validate_paths(&paths)?;
    let infos = process::inspect_file_locks(&paths)?;
    ui::display_file_locks(&infos);
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

pub fn handle_remove(opts: RemoveOptions) -> Result<()> {
    if opts.paths.is_empty() {
        bail!("Please specify at least one file or directory path");
    }

    fs_ops::validate_paths(&opts.paths)?;
    let files = fs_ops::collect_files_to_remove(&opts.paths, opts.recursive)?;

    if files.is_empty() {
        println!("No matching files or directories found");
        return Ok(());
    }

    if !ui::confirm_deletion(&files, opts.force || opts.anyway, opts.dry_run)? {
        let theme = Theme::new();
        println!("{}", theme.warn("Operation cancelled"));
        return Ok(());
    }

    // Check file locks and warn user
    if !ui::check_and_warn_file_locks(&files, opts.anyway)? {
        let theme = Theme::new();
        println!("{}", theme.warn("Operation cancelled"));
        return Ok(());
    }

    let results = fs_ops::remove_files(&files, opts.dry_run, opts.anyway);
    ui::display_removal_results(&results, opts.dry_run, opts.verbose);
    Ok(())
}
