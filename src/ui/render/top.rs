use crate::core::top::{ProcessView, TopRenderOptions};
use crate::ui::Theme;
use console::{Alignment, pad_str};
use std::io::{self, Write};

use super::{format_size, truncate_string};

/// Real-time process memory display
pub fn display_top(
    processes: &[ProcessView],
    opts: &TopRenderOptions,
    last_frame: &mut Vec<String>,
) {
    let theme = Theme::new();

    const RANK_W: usize = 4;
    const NAME_W: usize = 26;
    const PID_W: usize = 10;
    const MEM_W: usize = 10;
    const MEM_PCT_W: usize = 7;
    const CPU_W: usize = 8;

    let mut lines: Vec<String> = Vec::new();

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

    let mem_used_str = format_size(opts.used_memory);
    let mem_total_str = format_size(opts.total_memory);
    let mem_pct = if opts.total_memory > 0 {
        (opts.used_memory as f64 / opts.total_memory as f64) * 100.0
    } else {
        0.0
    };

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

        let mem_str = format_size(process.memory_bytes);
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

    let _ = write!(stdout, "\x1b[?25l");
    let _ = write!(stdout, "\x1b[H");

    let max_len = lines.len().max(last_frame.len());
    let mut changed_lines = 0;

    for i in 0..max_len {
        match (lines.get(i), last_frame.get(i)) {
            (Some(new_line), Some(old_line)) if new_line == old_line => {
                let _ = write!(stdout, "\x1b[E");
            }
            (Some(new_line), _) => {
                let _ = write!(stdout, "\x1b[2K{new_line}\r\n");
                changed_lines += 1;
            }
            (None, Some(_)) => {
                let _ = write!(stdout, "\x1b[2K\r\n");
                changed_lines += 1;
            }
            (None, None) => break,
        }
    }

    if max_len > lines.len() {
        let _ = write!(stdout, "\x1b[{}J", max_len - lines.len() + 1);
    } else {
        let _ = write!(stdout, "\x1b[J");
    }

    let _ = write!(stdout, "\x1b[?25h");
    let _ = stdout.flush();

    if changed_lines > 0 || lines.len() != last_frame.len() {
        last_frame.clear();
        last_frame.extend(lines.iter().cloned());
    }
}
