use crate::ui;
use crate::ui::TopRenderOptions;
use anyhow::Result;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

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

    // 进入备用屏幕，避免污染滚动历史（once 模式不需要）
    let use_alt_screen = !opts.once;
    if use_alt_screen {
        print!("\x1b[?1049h");
        let _ = io::stdout().flush();
    }

    let mut tick: u64 = 0;
    let mut last_frame: Vec<String> = Vec::new();
    let incremental = !opts.once;

    loop {
        tick = tick.wrapping_add(1);
        let start = Instant::now();

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

                ProcessView {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().into_owned(),
                    memory_bytes: memory,
                    memory_percent,
                    cpu: process.cpu_usage(),
                    cmd,
                }
            })
            .collect();

        processes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
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

        // 补偿刷新时间，保持接近 interval
        let elapsed = start.elapsed();
        let sleep_ms = ((opts.interval * 1000.0) as i64 - elapsed.as_millis() as i64).max(0) as u64;
        thread::sleep(Duration::from_millis(sleep_ms));
    }

    // 离开备用屏幕，恢复原屏幕内容
    if use_alt_screen {
        print!("\x1b[?1049l");
        let _ = io::stdout().flush();
    }

    Ok(())
}
