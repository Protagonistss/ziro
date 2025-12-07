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
        || env::var("TERM_PROGRAM").is_ok();
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

    // 安全降级条件：显式 plain，或非 UTF-8，或 Windows 且看起来非现代终端
    // 自动降级条件：
    // - 用户显式 plain
    // - 非 Windows 且非 UTF-8（大概率乱码）
    // - Windows 且「既不现代又非 UTF-8」（双重不安全）
    let should_degrade = profile.plain || (!is_windows && !utf8_ok) || (!utf8_ok && !looks_modern);
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

fn bool_to_flag(v: bool) -> &'static str {
    if v { "1" } else { "0" }
}

fn detect_windows_utf8() -> bool {
    if !cfg!(target_os = "windows") {
        return true;
    }

    if let Ok(output) = Command::new("cmd").args(["/C", "chcp"]).output() {
        if let Ok(text) = String::from_utf8(output.stdout) {
            return text.contains("65001");
        }
    }
    false
}
