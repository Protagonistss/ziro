use crate::cli::Cli;
use std::process::Command;
use std::{env, sync::OnceLock};

#[derive(Clone)]
pub struct TerminalProfile {
    pub plain: bool,
    pub ascii_icons: bool,
    pub no_color: bool,
    pub narrow: bool,
    pub alt_screen: bool,
    pub incremental: bool,
}

impl Default for TerminalProfile {
    fn default() -> Self {
        Self {
            plain: false,
            ascii_icons: false,
            no_color: false,
            narrow: false,
            alt_screen: true,
            incremental: true,
        }
    }
}

static GLOBAL_PROFILE: OnceLock<TerminalProfile> = OnceLock::new();

pub fn set_global_profile(profile: TerminalProfile) {
    let _ = GLOBAL_PROFILE.set(profile);
}

pub fn global_profile() -> TerminalProfile {
    GLOBAL_PROFILE
        .get()
        .cloned()
        .unwrap_or_else(TerminalProfile::default)
}

pub fn detect_profile(cli: &Cli) -> TerminalProfile {
    // 用户显式参数优先
    let mut profile = TerminalProfile {
        plain: cli.plain || is_truthy_env("ZIRO_PLAIN"),
        ascii_icons: cli.ascii || is_truthy_env("ZIRO_ASCII_ICONS"),
        no_color: cli.no_color || is_truthy_env("ZIRO_NO_COLOR") || is_truthy_env("NO_COLOR"),
        narrow: cli.narrow || is_truthy_env("ZIRO_NARROW"),
        ..TerminalProfile::default()
    };

    // 检测终端能力
    let is_windows = cfg!(target_os = "windows");
    let looks_modern = env::var("WT_SESSION").is_ok()
        || env::var("TERM")
            .map(|t| {
                let t = t.to_lowercase();
                t.contains("xterm")
                    || t.contains("cygwin")
                    || t.contains("screen")
                    || t.contains("tmux")
            })
            .unwrap_or(false)
        || env::var("ConEmuANSI").is_ok()
        || env::var("ANSICON").is_ok()
        || env::var("TERM_PROGRAM").is_ok()
        || is_modern_terminal();
    let utf8_ok = if is_windows {
        detect_windows_utf8()
            || env::var("LC_ALL")
                .or_else(|_| env::var("LANG"))
                .map(|v| v.to_lowercase().contains("65001") || v.to_lowercase().contains("utf-8"))
                .unwrap_or(false)
    } else {
        env::var("LC_ALL")
            .or_else(|_| env::var("LANG"))
            .map(|v| v.to_lowercase().contains("utf-8"))
            .unwrap_or(true)
    };

    // 改进的智能降级策略
    // 自动降级条件分析：
    // 1. 用户显式要求 plain 模式
    // 2. 非 Windows 系统且非 UTF-8 环境（大概率会乱码）
    // 3. Windows 系统下的不安全组合：
    //    - 既非 UTF-8 又非现代终端
    //    - Windows PowerShell 5.1 但不在现代终端中
    //    - 检测到传统控制台环境（conhost）
    let should_degrade = profile.plain
        || (!is_windows && !utf8_ok)
        || (is_windows && should_degrade_on_windows(utf8_ok, looks_modern));

    if should_degrade {
        profile.plain = true;
        profile.ascii_icons = true;
        profile.no_color = true;
        profile.narrow = true;
        profile.alt_screen = false;
        profile.incremental = false;
    }

    profile
}

pub fn apply_profile_env(profile: &TerminalProfile) {
    unsafe {
        env::set_var("ZIRO_PLAIN", bool_to_flag(profile.plain));
        env::set_var(
            "ZIRO_ASCII_ICONS",
            bool_to_flag(profile.ascii_icons || profile.plain),
        );
        env::set_var(
            "ZIRO_NO_COLOR",
            bool_to_flag(profile.no_color || profile.plain),
        );
        env::set_var("ZIRO_NARROW", bool_to_flag(profile.narrow || profile.plain));
    }
}

fn is_truthy_env(key: &str) -> bool {
    env::var(key)
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// 检测是否为现代终端（改进版本）
fn is_modern_terminal() -> bool {
    // 1. PowerShell 环境的精确检测
    if is_powershell_core() {
        // PowerShell Core 通常是现代的
        return true;
    }

    if is_windows_powershell_legacy() {
        // Windows PowerShell 5.1 在传统控制台中可能不支持 ANSI
        return is_windows_terminal_or_conemu();
    }

    // 2. 检查 Windows Terminal 或其他现代终端
    if is_windows_terminal_or_conemu() {
        return true;
    }

    // 3. 检查其他现代终端环境
    if env::var("TERM_PROGRAM")
        .map(|t| {
            let t = t.to_lowercase();
            matches!(
                t.as_str(),
                "vscode" | "hyper" | "terminus" | "windowsterminal" | "warp" | "wt"
            )
        })
        .unwrap_or(false)
    {
        return true;
    }

    false
}

/// 检测是否为 PowerShell Core (6+)
fn is_powershell_core() -> bool {
    // PowerShell Core 会在 PSVersionTable 中设置版本
    env::var("PSVersionTable").map(|_| true).unwrap_or(false)
}

/// 检测是否为 Windows PowerShell (5.1 及以下)
fn is_windows_powershell_legacy() -> bool {
    // Windows PowerShell 5.1 特有环境变量检测
    env::var("PSModulePath").is_ok()
        && env::var("PSVersionTable").is_err()
        && (env::var("PSHOME").is_ok() || env::var("PSExecutionPolicyPreference").is_ok())
}

/// 检测是否在 Windows Terminal 或 ConEmu 中运行
fn is_windows_terminal_or_conemu() -> bool {
    env::var("WT_SESSION").is_ok()  // Windows Terminal
        || env::var("ConEmuANSI").is_ok()  // ConEmu with ANSI
        || env::var("ANSICON").is_ok()     // ANSICON
        || env::var("TERM_PROGRAM").is_ok() // VSCode 等现代终端
}

/// Windows 环境下的降级决策函数
fn should_degrade_on_windows(utf8_ok: bool, looks_modern: bool) -> bool {
    // 情况1：既非 UTF-8 又非现代终端 -> 明确降级
    if !utf8_ok && !looks_modern {
        return true;
    }

    // 情况2：Windows PowerShell 5.1 且不在现代终端中
    if is_windows_powershell_legacy() && !is_windows_terminal_or_conemu() {
        return true;
    }

    // 情况3：检测到传统控制台环境（conhost）
    if is_traditional_console() {
        return true;
    }

    // 情况4：非 UTF-8 环境，即使看起来现代也要保守处理
    if !utf8_ok && !is_very_modern_terminal() {
        return true;
    }

    false
}

/// 检测是否为传统控制台（conhost）
fn is_traditional_console() -> bool {
    // 传统控制台通常没有这些现代环境变量
    env::var("WT_SESSION").is_err()
        && env::var("TERM_PROGRAM").is_err()
        && env::var("ConEmuANSI").is_err()
        && env::var("ANSICON").is_err()
        && (env::var("TERM").is_err() || env::var("TERM").unwrap_or_default().is_empty())
}

/// 检测是否为非常现代的终端（值得冒险尝试 ANSI）
fn is_very_modern_terminal() -> bool {
    env::var("WT_SESSION").is_ok()  // Windows Terminal
        || (env::var("TERM_PROGRAM").is_ok()
            && env::var("TERM_PROGRAM").unwrap_or_default().to_lowercase().contains("vscode"))
}

fn bool_to_flag(v: bool) -> &'static str {
    if v { "1" } else { "0" }
}

fn detect_windows_utf8() -> bool {
    if !cfg!(target_os = "windows") {
        return true;
    }

    // 方法1: 检查活动代码页
    if let Ok(output) = Command::new("cmd").args(["/C", "chcp"]).output() {
        if let Ok(text) = String::from_utf8(output.stdout) {
            // 查找 "活动代码页: 65001" 或类似模式
            if text.contains("65001") {
                return true;
            }
        }
    }

    // 方法2: 检查系统默认输出代码页
    if let Ok(output) = Command::new("cmd").args(["/C", "echo %LANG%"]).output() {
        if let Ok(lang) = String::from_utf8(output.stdout) {
            let lang = lang.trim().to_lowercase();
            if lang.contains("utf-8") || lang.contains("65001") {
                return true;
            }
        }
    }

    // 方法3: 检查系统环境变量
    if let Ok(locale) = env::var("LC_ALL").or_else(|_| env::var("LANG")) {
        let locale = locale.to_lowercase();
        if locale.contains("utf-8") || locale.contains("65001") {
            return true;
        }
    }

    // 方法4: 检查 Windows Terminal 或其他现代终端
    if env::var("WT_SESSION")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        return true;
    }

    // 方法5: 检查终端程序
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        if [
            "vscode",
            "hyper",
            "terminus",
            "windowsterminal",
            "warp",
            "wt",
        ]
        .contains(&term_program.to_lowercase().as_str())
        {
            return true;
        }
    }

    // 方法6: 检查 TERM 变量
    if let Ok(term) = env::var("TERM") {
        let term = term.to_lowercase();
        if term.contains("xterm") || term.contains("screen") || term.contains("tmux") {
            return true;
        }
    }

    // 默认保守策略
    false
}
