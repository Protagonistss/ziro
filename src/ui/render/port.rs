use crate::core::port::PortInfo;
use crate::ui::Theme;
use anyhow::Result;
use inquire::{Confirm, MultiSelect};

use super::{format_size, tree_branches, truncate_string};

/// Display message for port not in use
pub fn display_port_not_found(port: u16) {
    let theme = Theme::new();
    println!("{}", theme.warn(format!("Port {port} is not in use")));
}

/// Display multiple port info with interactive selection
pub fn select_processes_to_kill(port_infos: Vec<PortInfo>) -> Result<Vec<PortInfo>> {
    let theme = Theme::new();

    if port_infos.is_empty() {
        println!(
            "{}",
            theme.warn("No processes found occupying the specified ports")
        );
        return Ok(vec![]);
    }

    let options: Vec<String> = port_infos
        .iter()
        .enumerate()
        .map(|(i, info)| {
            format!(
                "[{}] Port {} - {} (PID: {})",
                i, info.port, info.process.name, info.process.pid
            )
        })
        .collect();

    let defaults: Vec<usize> = (0..options.len()).collect();

    let selected = MultiSelect::new("Select processes to kill:", options)
        .with_default(&defaults)
        .prompt()?;

    let result: Vec<PortInfo> = selected
        .iter()
        .filter_map(|s| {
            let idx_str = s.trim_start_matches('[').split(']').next()?;
            let idx: usize = idx_str.parse().ok()?;
            port_infos.get(idx).cloned()
        })
        .collect();

    if result.is_empty() {
        println!("{}", theme.warn("No processes selected"));
        return Ok(vec![]);
    }

    let confirm = Confirm::new("Confirm killing these processes?")
        .with_default(false)
        .prompt()?;

    if confirm {
        Ok(result)
    } else {
        println!("{}", theme.warn("Operation cancelled"));
        Ok(vec![])
    }
}

/// Display kill results
pub fn display_kill_results(results: &[(u32, Result<()>)]) {
    let theme = Theme::new();

    for (pid, result) in results {
        match result {
            Ok(()) => println!(
                "{} {}",
                theme.icon_success(),
                theme.success(format!("Successfully killed process {pid}"))
            ),
            Err(e) => println!(
                "{} {}: {}",
                theme.icon_error(),
                theme.error(format!("Failed to kill process {pid}")),
                e
            ),
        }
    }
}

/// Display multiple port info in tree structure
pub fn display_ports_tree(ports: &[u16], port_infos: Vec<PortInfo>) {
    if ports.is_empty() {
        return;
    }

    let theme = Theme::new();

    println!(
        "{} {}",
        theme.icon_lightning(),
        theme.title("Port Query Results")
    );
    println!();

    let mut port_map = std::collections::HashMap::new();
    for info in port_infos {
        port_map.insert(info.port, info);
    }

    let total = ports.len();
    for (index, &port) in ports.iter().enumerate() {
        let (branch, continuation) = tree_branches(total, index);

        if let Some(info) = port_map.get(&port) {
            println!(
                "{} {} {}",
                branch,
                theme.highlight(port.to_string()),
                theme.icon_success()
            );

            println!(
                "{}├─ {}: {} ({})",
                continuation,
                theme.info("Process"),
                theme.success(&info.process.name),
                theme.muted(info.process.pid.to_string())
            );

            let cmd = truncate_string(&info.process.cmd.join(" "), 60);
            println!(
                "{}├─ {}: {}",
                continuation,
                theme.info("Command"),
                theme.muted(cmd)
            );

            println!(
                "{}└─ {}: {} CPU, {} Memory",
                continuation,
                theme.info("Resources"),
                theme.accent(format!("{:.1}%", info.process.cpu_usage)),
                theme.accent(format_size(info.process.memory))
            );
        } else {
            println!(
                "{} {} {} {}",
                branch,
                theme.highlight(port.to_string()),
                theme.icon_error(),
                theme.muted("(free)")
            );
        }

        if continuation == "│  " {
            println!("{continuation}");
        }
    }
}

/// Display all port usage in tree structure (for list command)
pub fn display_ports_tree_all(port_infos: Vec<PortInfo>) {
    let theme = Theme::new();

    if port_infos.is_empty() {
        println!("{}", theme.warn("No ports are currently in use"));
        return;
    }

    println!(
        "{} {} {}",
        theme.icon_lightning(),
        theme.title("Port Usage"),
        theme.muted(format!("({} total)", port_infos.len()))
    );
    println!();

    let total = port_infos.len();
    for (index, info) in port_infos.iter().enumerate() {
        let (branch, continuation) = tree_branches(total, index);

        println!(
            "{} {} {}",
            branch,
            theme.highlight(info.port.to_string()),
            theme.icon_success()
        );

        println!(
            "{}├─ {}: {} ({})",
            continuation,
            theme.info("Process"),
            theme.success(&info.process.name),
            theme.muted(info.process.pid.to_string())
        );

        let cmd = truncate_string(&info.process.cmd.join(" "), 60);
        println!(
            "{}├─ {}: {}",
            continuation,
            theme.info("Command"),
            theme.muted(cmd)
        );

        println!(
            "{}└─ {}: {} CPU, {} Memory",
            continuation,
            theme.info("Resources"),
            theme.accent(format!("{:.1}%", info.process.cpu_usage)),
            theme.accent(format_size(info.process.memory))
        );

        if continuation == "│  " {
            println!("{continuation}");
        }
    }
}

/// Display force kill results
pub fn display_kill_results_force(port_infos: &[PortInfo], results: &[(u32, Result<()>)]) {
    let theme = Theme::new();

    println!(
        "{} {}",
        theme.icon_fire(),
        theme.error_bold("Force Kill Processes")
    );
    println!();

    println!("{}", theme.title("Target processes:"));
    for info in port_infos {
        println!(
            "  Port {} - {} (PID: {})",
            theme.highlight(info.port.to_string()),
            theme.success(&info.process.name),
            theme.muted(info.process.pid.to_string())
        );
    }
    println!();

    println!("{}", theme.title("Kill results:"));
    let mut success_count = 0;
    let mut error_count = 0;

    for (pid, result) in results {
        match result {
            Ok(()) => {
                success_count += 1;
                println!(
                    "{} {}",
                    theme.icon_success(),
                    theme.success(format!("Successfully force-killed process {pid}"))
                );
            }
            Err(e) => {
                error_count += 1;
                println!(
                    "{} {}: {}",
                    theme.icon_error(),
                    theme.error(format!("Failed to force-kill process {pid}")),
                    e
                );
            }
        }
    }

    println!();
    println!(
        "{} {} {}",
        theme.title("Force kill complete"),
        theme.success(format!("Success: {success_count}")),
        theme.error(format!("Failed: {error_count}"))
    );
}
