use crate::platform::term::{self, TerminalProfile};
#[cfg(target_os = "windows")]
use crate::platform::term::{is_powershell_core, is_windows_powershell_legacy, is_windows_terminal_or_conemu};
use crate::ui;
use crate::ui::TopRenderOptions;
use anyhow::Result;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

/// Check whether alternate screen should be used (improved version)
fn should_use_alt_screen(profile: &TerminalProfile) -> bool {
    if !profile.alt_screen {
        return false;
    }

    // Use improved terminal detection logic
    #[cfg(target_os = "windows")]
    {
        // Windows Terminal explicitly supports alternate screen
        if std::env::var("WT_SESSION").is_ok() {
            return true;
        }

        // VSCode terminal supports alternate screen
        if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
            if term_program.to_lowercase().contains("vscode") {
                return true;
            }
        }

        // ConEmu and other modern terminals with ANSI support
        if std::env::var("ConEmuANSI").is_ok() || std::env::var("ANSICON").is_ok() {
            return true;
        }

        // PowerShell Core usually supports alternate screen
        if is_powershell_core() {
            return true;
        }

        // Directly detect Warp terminal and other modern terminals
        if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
            let term_program = term_program.to_lowercase();
            if term_program.contains("warp") || term_program.contains("warpterminal") {
                return true;
            }
        }

        // TERM variable detection - support xterm-256color and other modern terminals
        if let Ok(term) = std::env::var("TERM") {
            let term = term.to_lowercase();
            if term.contains("xterm-256color")
                || term.contains("xterm")
                || term.contains("256color")
            {
                return true;
            }
        }

        // Windows PowerShell 5.1 only supports alternate screen in specific environments
        if is_windows_powershell_legacy() {
            // Only use alternate screen in Windows Terminal or ConEmu
            return is_windows_terminal_or_conemu();
        }

        // Default: don't use alternate screen to avoid issues
        false
    }

    // Non-Windows systems usually support alternate screen
    #[cfg(not(target_os = "windows"))]
    true
}

/// Safely enter alternate screen
fn enter_alternate_screen() {
    // Clear screen and move to top first
    print!("\x1b[2J\x1b[H");

    // Try to enter alternate screen
    print!("\x1b[?1049h");

    // Hide cursor
    print!("\x1b[?25l");

    let _ = io::stdout().flush();
}

/// Safely exit alternate screen
fn exit_alternate_screen() {
    // Show cursor
    print!("\x1b[?25h");

    // Exit alternate screen
    print!("\x1b[?1049l");

    let _ = io::stdout().flush();
}

/// Top subcommand options
pub struct TopOptions {
    pub interval: f32,
    pub limit: usize,
    pub show_cpu: bool,
    pub show_cmd: bool,
    pub once: bool,
}

/// Process info for display
pub struct ProcessView {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
    pub memory_percent: f64,
    pub cpu: f32,
    pub cmd: String,
}

pub fn run_top(opts: TopOptions) -> Result<()> {
    let process_refresh = ProcessRefreshKind::everything();
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes(process_refresh));

    // Decide whether to use alternate screen / incremental refresh based on terminal capabilities
    let profile = term::global_profile();
    let use_alt_screen = !opts.once && should_use_alt_screen(&profile);
    let incremental = !opts.once && profile.incremental;

    // Enter alternate screen to avoid polluting scroll history (not needed for once mode)
    if use_alt_screen {
        enter_alternate_screen();
    }

    let mut tick: u64 = 0;
    let mut last_frame: Vec<String> = Vec::new();

    // Initial refresh to establish baseline CPU usage
    system.refresh_processes_specifics(ProcessesToUpdate::All, process_refresh);
    system.refresh_memory();

    // Wait a short time for better CPU usage calculation
    if !opts.once {
        thread::sleep(Duration::from_millis(100));
    }

    loop {
        tick = tick.wrapping_add(1);
        let start = Instant::now();

        // Use smarter refresh strategy
        system.refresh_processes_specifics(ProcessesToUpdate::All, process_refresh);
        system.refresh_memory();

        let total_memory = system.total_memory();
        let used_memory = system.used_memory();

        let mut processes: Vec<ProcessView> = system
            .processes()
            .iter()
            .map(|(pid, process)| {
                let cmd = process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ");

                let memory = process.memory();
                let memory_percent = if total_memory > 0 {
                    (memory as f64 / total_memory as f64) * 100.0
                } else {
                    0.0
                };

                // CPU usage calculation - use more stable values
                let cpu_usage = process.cpu_usage();

                ProcessView {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().into_owned(),
                    memory_bytes: memory,
                    memory_percent,
                    cpu: cpu_usage,
                    cmd,
                }
            })
            .collect();

        // Sort by memory usage, but factor in CPU usage weight
        if opts.show_cpu {
            processes.sort_by(|a, b| {
                let score_a = a.memory_bytes as f64 * 0.7 + a.cpu as f64 * 1000.0 * 0.3;
                let score_b = b.memory_bytes as f64 * 0.7 + b.cpu as f64 * 1000.0 * 0.3;
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            processes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
        }

        processes.truncate(opts.limit.max(1));

        let render_opts = TopRenderOptions {
            total_memory,
            used_memory,
            refresh: tick,
            interval: opts.interval,
            show_cpu: opts.show_cpu,
            show_cmd: opts.show_cmd,
            incremental,
        };

        ui::display_top(&processes, render_opts, &mut last_frame);

        if opts.once {
            break;
        }

        // More precise refresh timing control
        let elapsed = start.elapsed();
        let target_duration = Duration::from_secs_f32(opts.interval);

        if elapsed < target_duration {
            let remaining = target_duration - elapsed;
            thread::sleep(remaining);
        }
    }

    // Leave alternate screen and restore original screen content
    if use_alt_screen {
        exit_alternate_screen();
    }

    Ok(())
}
