use crate::term;
use crate::ui;
use crate::ui::TopRenderOptions;
use anyhow::Result;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

use crate::term::TerminalProfile;

/// 检查是否应该使用备用屏幕
fn should_use_alt_screen(profile: &TerminalProfile) -> bool {
    if !profile.alt_screen {
        return false;
    }

    // 在某些终端中，备用屏幕可能不稳定，需要额外检查
    #[cfg(target_os = "windows")]
    {
        // 在 PowerShell 中，备用屏幕支持可能不稳定
        if std::env::var("PSModulePath").is_ok() {
            // PowerShell 检测 - 保守策略
            return is_power_shell_conhost();
        }

        // Windows Terminal 支持
        if std::env::var("WT_SESSION").is_ok() {
            return true;
        }

        // 其他现代终端
        if std::env::var("TERM_PROGRAM").is_ok()
            || std::env::var("ConEmuANSI").is_ok()
            || std::env::var("ANSICON").is_ok()
        {
            return true;
        }
    }

    // 非 Windows 系统通常支持备用屏幕
    #[cfg(not(target_os = "windows"))]
    {
        return true;
    }

    false
}

/// 检测是否为 PowerShell 在 ConHost 中运行
#[cfg(target_os = "windows")]
fn is_power_shell_conhost() -> bool {
    // 检查 TERM 环境变量，如果不存在或者为空，可能是在 conhost 中
    if let Ok(term) = std::env::var("TERM") {
        if term.is_empty() || term.to_lowercase().contains("conhost") {
            return false; // conhost 不支持备用屏幕
        }
    } else {
        return false; // 没有 TERM 变量，可能是传统控制台
    }

    // 如果有 WT_SESSION，说明在 Windows Terminal 中
    if std::env::var("WT_SESSION").is_ok() {
        return true;
    }

    // 其他情况下假设支持
    true
}

/// 安全地进入备用屏幕
fn enter_alternate_screen() {
    // 先清除屏幕并移动到顶部
    print!("\x1b[2J\x1b[H");

    // 尝试进入备用屏幕
    print!("\x1b[?1049h");

    // 隐藏光标
    print!("\x1b[?25l");

    let _ = io::stdout().flush();
}

/// 安全地退出备用屏幕
fn exit_alternate_screen() {
    // 显示光标
    print!("\x1b[?25h");

    // 退出备用屏幕
    print!("\x1b[?1049l");

    let _ = io::stdout().flush();
}

/// top 子命令的配置
pub struct TopOptions {
    pub interval: f32,
    pub limit: usize,
    pub show_cpu: bool,
    pub show_cmd: bool,
    pub once: bool,
}

/// 用于展示的进程信息
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

    // 根据终端能力决定是否使用备用屏幕 / 增量刷新，避免在不支持的控制台显示乱码
    let profile = term::global_profile();
    let use_alt_screen = !opts.once && should_use_alt_screen(&profile);
    let incremental = !opts.once && profile.incremental;

    // 进入备用屏幕，避免污染滚动历史（once 模式不需要）
    if use_alt_screen {
        enter_alternate_screen();
    }

    let mut tick: u64 = 0;
    let mut last_frame: Vec<String> = Vec::new();

    // 初始刷新以建立基准 CPU 使用率
    system.refresh_processes_specifics(ProcessesToUpdate::All, process_refresh);
    system.refresh_memory();

    // 为更好的 CPU 使用率计算，等待一小段时间
    if !opts.once {
        thread::sleep(Duration::from_millis(100));
    }

    loop {
        tick = tick.wrapping_add(1);
        let start = Instant::now();

        // 使用更智能的刷新策略
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

                // CPU 使用率计算 - 使用更稳定的值
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

        // 按内存使用率排序，但考虑 CPU 使用率的权重
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

        // 更精确的刷新时间控制
        let elapsed = start.elapsed();
        let target_duration = Duration::from_secs_f32(opts.interval);

        if elapsed < target_duration {
            let remaining = target_duration - elapsed;
            thread::sleep(remaining);
        }
    }

    // 离开备用屏幕，恢复原屏幕内容
    if use_alt_screen {
        exit_alternate_screen();
    }

    Ok(())
}
