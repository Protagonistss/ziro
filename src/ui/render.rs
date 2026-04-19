use crate::core::fs_ops::FileInfo;
use crate::core::port::PortInfo;
use crate::core::process::FileLockInfo;
use crate::core::top::ProcessView;
use crate::ui::Theme;
use anyhow::Result;
use console::{Alignment, pad_str};
use inquire::{Confirm, MultiSelect};
use std::io::{self, Write};

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

    // Confirm operation
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

/// Truncate string to specified length
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let end = s
            .char_indices()
            .map(|(i, _)| i)
            .nth(max_len.saturating_sub(3))
            .unwrap_or(s.len());
        format!("{}...", &s[..end])
    }
}

/// Return tree drawing characters for the given position
fn tree_branches(total: usize, index: usize) -> (&'static str, &'static str) {
    let is_last = index == total - 1;
    if is_last {
        ("└─", "   ")
    } else {
        ("├─", "│  ")
    }
}

/// Display error message
pub fn display_error(error: &anyhow::Error) {
    let theme = Theme::new();
    eprintln!("{} {}", theme.error_bold("Error:"), error);
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

    // Build port to process info mapping
    let mut port_map = std::collections::HashMap::new();
    for info in port_infos {
        port_map.insert(info.port, info);
    }

    let total = ports.len();
    for (index, &port) in ports.iter().enumerate() {
        let (branch, continuation) = tree_branches(total, index);

        if let Some(info) = port_map.get(&port) {
            // Port in use
            println!(
                "{} {} {}",
                branch,
                theme.highlight(port.to_string()),
                theme.icon_success()
            );

            // Process info
            println!(
                "{}├─ {}: {} ({})",
                continuation,
                theme.info("Process"),
                theme.success(&info.process.name),
                theme.muted(info.process.pid.to_string())
            );

            // Command
            let cmd = truncate_string(&info.process.cmd.join(" "), 60);
            println!(
                "{}├─ {}: {}",
                continuation,
                theme.info("Command"),
                theme.muted(cmd)
            );

            // Resource usage
            println!(
                "{}└─ {}: {} CPU, {} Memory",
                continuation,
                theme.info("Resources"),
                theme.accent(format!("{:.1}%", info.process.cpu_usage)),
                theme.accent(super::format_size(info.process.memory))
            );
        } else {
            // Port free
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

        // Port number and status
        println!(
            "{} {} {}",
            branch,
            theme.highlight(info.port.to_string()),
            theme.icon_success()
        );

        // Process info
        println!(
            "{}├─ {}: {} ({})",
            continuation,
            theme.info("Process"),
            theme.success(&info.process.name),
            theme.muted(info.process.pid.to_string())
        );

        // Command
        let cmd = truncate_string(&info.process.cmd.join(" "), 60);
        println!(
            "{}├─ {}: {}",
            continuation,
            theme.info("Command"),
            theme.muted(cmd)
        );

        // Resource usage
        println!(
            "{}└─ {}: {} CPU, {} Memory",
            continuation,
            theme.info("Resources"),
            theme.accent(format!("{:.1}%", info.process.cpu_usage)),
            theme.accent(super::format_size(info.process.memory))
        );

        if continuation == "│  " {
            println!("{continuation}");
        }
    }
}

/// Display file/directory lock status
pub fn display_file_locks(infos: &[FileLockInfo]) {
    let theme = Theme::new();

    if infos.is_empty() {
        println!("{}", theme.warn("No paths found to check"));
        return;
    }

    println!("{} {}", theme.icon_search(), theme.title("File Lock Query"));
    println!();

    let total = infos.len();
    for (index, info) in infos.iter().enumerate() {
        let (branch, continuation) = tree_branches(total, index);

        let kind = if info.path.is_dir() {
            theme.blue("Dir")
        } else {
            theme.success("File")
        };

        let status = if info.locked {
            theme.error("Locked")
        } else {
            theme.success("Free")
        };

        println!(
            "{branch} {} {} {}",
            theme.highlight(info.path.display().to_string()),
            kind,
            status
        );

        if info.processes.is_empty() {
            if info.locked {
                println!(
                    "{continuation}└─ {}",
                    theme.warn("No locking process found, may need admin privileges or handle.exe")
                );
            }
        } else {
            let proc_total = info.processes.len();
            for (proc_index, proc_info) in info.processes.iter().enumerate() {
                let proc_last = proc_index == proc_total - 1;
                let proc_branch = if proc_last { "└─" } else { "├─" };
                let proc_continuation = if proc_last { "   " } else { "│  " };

                println!(
                    "{continuation}{proc_branch} {} {} ({})",
                    theme.info("Process"),
                    theme.success(&proc_info.name),
                    theme.muted(format!("PID: {}", proc_info.pid))
                );

                if !proc_info.cmd.is_empty() {
                    println!(
                        "{continuation}{proc_continuation} {} {}",
                        theme.info("Command"),
                        theme.muted(truncate_string(&proc_info.cmd, 80))
                    );
                }
            }
        }

        if continuation == "│  " {
            println!("{continuation}");
        }
    }
}

/// Display deletion preview
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
        theme.title("Summary:"),
        theme.success(format!("{file_count} files")),
        theme.blue(format!("{dir_count} directories")),
        theme.warn(format!("Total size: {}", super::format_size(total_size)))
    );
    println!();

    // Display file list preview
    let total = files.len().min(10);
    for file in files.iter().take(total) {
        let icon = if file.is_dir {
            theme.icon_folder()
        } else if file.is_symlink {
            theme.icon_link()
        } else {
            theme.icon_file()
        };

        let size_str = if !file.is_dir && !file.is_symlink {
            let size = format!(" ({})", super::format_size(file.size));
            theme.muted(size)
        } else {
            String::new()
        };

        let file_type = if file.is_dir {
            theme.blue("Dir")
        } else if file.is_symlink {
            theme.accent("Symlink")
        } else {
            theme.success("File")
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
            theme.muted(format!("  ... {} more items", files.len() - 10))
        );
    }

    println!();
}

/// Confirm deletion operation
pub fn confirm_deletion(files: &[FileInfo], skip_confirm: bool, dry_run: bool) -> Result<bool> {
    let theme = Theme::new();

    if dry_run {
        println!(
            "{} {}",
            theme.icon_search(),
            theme.info_bold("Preview mode - no files will be deleted")
        );
        display_deletion_preview(files);
        return Ok(true);
    }

    if skip_confirm {
        return Ok(true);
    }

    println!(
        "{} {}",
        theme.icon_warning(),
        theme.error_bold("About to delete the following")
    );
    display_deletion_preview(files);

    let confirm = Confirm::new("Confirm deleting these items? This cannot be undone!")
        .with_default(false)
        .with_help_message("Use --force to skip this confirmation")
        .prompt()?;

    Ok(confirm)
}

/// Check file locks and handle them
///
/// - Default: show lock info and ask user whether to continue
/// - `--anyway`: auto-kill locking processes, then continue
///
/// Returns true to proceed with deletion, false to cancel
pub fn check_and_warn_file_locks(files: &[FileInfo], anyway: bool) -> Result<bool> {
    use crate::core::process::{inspect_file_locks, kill_processes_force};
    use std::path::PathBuf;

    let theme = Theme::new();

    // Extract all file paths
    let paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();

    // Check file locks
    let lock_infos = match inspect_file_locks(&paths) {
        Ok(infos) => infos,
        Err(e) => {
            // Check failed: warn but allow to continue
            eprintln!(
                "{} {}: {}",
                theme.icon_warning(),
                theme.warn("Unable to check file locks"),
                e
            );
            eprintln!("{}", theme.muted("Will proceed with deletion"));
            return Ok(true);
        }
    };

    // Filter to only locked files
    let locked_files: Vec<FileLockInfo> = lock_infos
        .into_iter()
        .filter(|info| info.locked || !info.processes.is_empty())
        .collect();

    // No locks found, proceed
    if locked_files.is_empty() {
        return Ok(true);
    }

    // With --anyway: auto-kill locking processes
    if anyway {
        // Collect all locking process PIDs
        let mut pids = Vec::new();
        for info in &locked_files {
            for proc in &info.processes {
                if !pids.contains(&proc.pid) {
                    pids.push(proc.pid);
                }
            }
        }

        // Show processes to be killed
        println!();
        println!(
            "{} {}",
            theme.icon_warning(),
            theme.error_bold("Files locked, killing locking processes...")
        );
        println!();
        display_file_locks(&locked_files);
        println!();

        // Force kill processes
        let results = kill_processes_force(&pids);

        // Show kill results
        for (pid, result) in results {
            match result {
                Ok(_) => {
                    println!(
                        "{} {}",
                        theme.icon_success(),
                        theme.muted(format!("Killed process PID: {pid}"))
                    );
                }
                Err(e) => {
                    println!(
                        "{} {}",
                        theme.icon_error(),
                        theme.error(format!("Failed to kill process PID {pid}: {e}"))
                    );
                }
            }
        }
        println!();

        return Ok(true);
    }

    // Default: show lock info and ask user
    println!();
    println!(
        "{} {}",
        theme.icon_warning(),
        theme.error_bold("Files are locked")
    );
    println!();

    // Show detailed info using display_file_locks
    display_file_locks(&locked_files);

    println!();

    // Ask user whether to continue
    let confirm = Confirm::new("These files are in use, continue trying to delete?")
        .with_default(false)
        .with_help_message("Use --anyway to auto-kill locking processes and delete")
        .prompt()?;

    Ok(confirm)
}

/// Display deletion results
pub fn display_removal_results(
    results: &[(std::path::PathBuf, Result<()>)],
    dry_run: bool,
    verbose: bool,
) {
    let theme = Theme::new();
    let action = if dry_run { "Preview" } else { "Delete" };
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

    // Non-verbose mode: show summary only
    if !verbose {
        println!(
            "{} {} {}",
            theme.title("Done"),
            theme.success(format!("Success: {success_count}")),
            theme.error(format!("Failed: {error_count}"))
        );

        // Only show failed files when there are errors
        if error_count > 0 {
            for (path, result) in results {
                if let Err(e) = result {
                    println!(
                        "{} {} {}",
                        theme.icon_error(),
                        theme.error(format!("Failed to delete {}", path.display())),
                        e
                    );
                }
            }
        }
        return;
    }

    // Verbose mode: show all details
    println!(
        "{} {} {}",
        theme.title("Done"),
        theme.success(format!("Success: {success_count}")),
        theme.error(format!("Failed: {error_count}"))
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
                theme.error(format!("Failed to delete {}", path.display())),
                e
            ),
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

    // Show target processes first
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

    // Show kill results
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

/// Real-time process memory display
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

    // Column width config (using console::pad_str, supports CJK wide characters)
    const RANK_W: usize = 4;
    const NAME_W: usize = 26;
    const PID_W: usize = 10;
    const MEM_W: usize = 10;
    const MEM_PCT_W: usize = 7;
    const CPU_W: usize = 8;

    let mut lines: Vec<String> = Vec::new();

    // Top summary with live status indicator
    let status_icon = if opts.refresh.is_multiple_of(2) {
        "●"
    } else {
        "◐"
    };
    lines.push(format!(
        "{} {} {}",
        theme.icon_lightning(),
        theme.title("Process Memory Usage"),
        theme.muted(format!("[{status_icon}]"))
    ));

    let mem_used_str = super::format_size(opts.used_memory);
    let mem_total_str = super::format_size(opts.total_memory);
    let mem_pct = if opts.total_memory > 0 {
        (opts.used_memory as f64 / opts.total_memory as f64) * 100.0
    } else {
        0.0
    };

    // Build live status line
    let status_line = format!(
        "Refresh: {} | Interval: {:.1}s | Processes: {} | Memory: {} / {} ({:.1}%) | {}",
        opts.refresh,
        opts.interval,
        processes.len(),
        mem_used_str,
        mem_total_str,
        mem_pct,
        theme.muted("Ctrl+C to exit")
    );
    lines.push(status_line);

    // Add progress bar for memory usage
    let bar_width = 30;
    let filled = (mem_pct / 100.0 * bar_width as f64).round() as usize;
    let bar = "=".repeat(filled) + &"·".repeat(bar_width - filled);
    lines.push(theme.muted(format!("[{bar}]")).to_string());
    lines.push(String::new());

    let header_rank = pad_str("#", RANK_W, Alignment::Left, None);
    let header_name = pad_str("Name", NAME_W, Alignment::Left, None);
    let header_pid = pad_str("PID", PID_W, Alignment::Left, None);
    let header_mem = pad_str("Memory", MEM_W, Alignment::Right, None);
    let header_mem_pct = pad_str("Mem%", MEM_PCT_W, Alignment::Right, None);
    let header_cpu = pad_str("CPU", CPU_W, Alignment::Right, None);
    let header_cmd = if opts.show_cmd { "Command" } else { "" };

    lines.push(format!(
        "{header_rank} {header_name} {header_pid} {header_mem} {header_mem_pct} {header_cpu} {header_cmd}"
    ));

    let sep_len = RANK_W + NAME_W + PID_W + MEM_W + MEM_PCT_W + CPU_W + 6;
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

        let mem_str = super::format_size(process.memory_bytes);
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

/// Render built lines to terminal incrementally
fn render_frame(lines: &[String], incremental: bool, last_frame: &mut Vec<String>) {
    if !incremental {
        for line in lines {
            println!("{line}");
        }
        return;
    }

    let mut stdout = io::stdout();

    // Hide cursor to avoid flicker
    let _ = write!(stdout, "\x1b[?25l");

    // Move to top-left using efficient control sequence
    let _ = write!(stdout, "\x1b[H");

    let max_len = lines.len().max(last_frame.len());
    let mut changed_lines = 0;

    for i in 0..max_len {
        match (lines.get(i), last_frame.get(i)) {
            (Some(new_line), Some(old_line)) if new_line == old_line => {
                // Content unchanged, move to next line quickly
                let _ = write!(stdout, "\x1b[E");
            }
            (Some(new_line), _) => {
                // New or changed line, clear and output
                let _ = write!(stdout, "\x1b[2K{new_line}\r\n");
                changed_lines += 1;
            }
            (None, Some(_)) => {
                // Old line needs clearing
                let _ = write!(stdout, "\x1b[2K\r\n");
                changed_lines += 1;
            }
            (None, None) => break,
        }
    }

    // Only clear excess lines to reduce unnecessary operations
    if max_len > lines.len() {
        let _ = write!(stdout, "\x1b[{}J", max_len - lines.len() + 1);
    } else {
        // Clear from cursor to end of screen
        let _ = write!(stdout, "\x1b[J");
    }

    // Show cursor
    let _ = write!(stdout, "\x1b[?25h");

    // Flush output buffer immediately
    let _ = stdout.flush();

    // Efficiently update last_frame
    if changed_lines > 0 || lines.len() != last_frame.len() {
        last_frame.clear();
        last_frame.extend(lines.iter().cloned());
    }
}
