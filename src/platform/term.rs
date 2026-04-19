use crate::cli::Cli;
use std::process::Command;
use std::{env, sync::OnceLock};

#[derive(Clone, Debug)]
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
    // User-explicit arguments take priority
    let mut profile = TerminalProfile {
        plain: cli.plain || is_truthy_env("ZIRO_PLAIN"),
        ascii_icons: cli.ascii || is_truthy_env("ZIRO_ASCII_ICONS"),
        no_color: cli.no_color || is_truthy_env("ZIRO_NO_COLOR") || is_truthy_env("NO_COLOR"),
        narrow: cli.narrow || is_truthy_env("ZIRO_NARROW"),
        ..TerminalProfile::default()
    };

    // Detect terminal capabilities
    let is_windows = cfg!(target_os = "windows");
    let vt_supported = has_virtual_terminal_processing();

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
        || vt_supported
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

    // Improved smart degradation strategy
    // Auto-degradation condition analysis:
    // 1. User explicitly requests plain mode
    // 2. Non-Windows system without UTF-8 environment (likely garbled output)
    // 3. Unsafe combinations on Windows:
    //    - Neither UTF-8 nor modern terminal
    //    - Windows PowerShell 5.1 not running inside a modern terminal
    //    - Legacy console environment detected (conhost)
    let should_degrade = profile.plain
        || (!is_windows && !utf8_ok)
        || (is_windows && should_degrade_on_windows(utf8_ok, looks_modern, vt_supported));

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

pub fn is_truthy_env(key: &str) -> bool {
    env::var(key).map(|v| is_truthy(&v)).unwrap_or(false)
}

/// Check if a string value represents a truthy/affirmative value
pub fn is_truthy(value: &str) -> bool {
    matches!(value.to_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

/// Detect whether the terminal is modern (improved version)
fn is_modern_terminal() -> bool {
    // 1. Precise PowerShell environment detection
    if is_powershell_core() {
        // PowerShell Core is generally modern
        return true;
    }

    if is_windows_powershell_legacy() {
        // Windows PowerShell 5.1 may not support ANSI in legacy consoles
        return is_windows_terminal_or_conemu();
    }

    // 2. Check for Windows Terminal or other modern terminals
    if is_windows_terminal_or_conemu() {
        return true;
    }

    // 3. Broader modern terminal detection
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        let term_program = term_program.to_lowercase();
        // Support more modern terminals
        if [
            "vscode",
            "hyper",
            "terminus",
            "windowsterminal",
            "warp",
            "wt",
            "warpterminal",
            "iterm",
            "alacritty",
            "kitty",
            "wezterm",
        ]
        .contains(&term_program.as_str())
        {
            return true;
        }
    }

    // 4. TERM variable detection (Unix-like terminal emulators)
    if let Ok(term) = env::var("TERM") {
        let term = term.to_lowercase();
        if term.contains("xterm")
            || term.contains("screen")
            || term.contains("tmux")
            || term.contains("256color")
            || term.contains("alacritty")
            || term.contains("kitty")
        {
            return true;
        }
    }

    // 5. Other modern terminal indicators
    if env::var("COLORTERM").is_ok() || env::var("TERM_PROGRAM_VERSION").is_ok() {
        return true;
    }

    false
}

/// Detect whether the terminal is PowerShell Core (6+)
pub fn is_powershell_core() -> bool {
    // PowerShell Core sets the version in PSVersionTable
    env::var("PSVersionTable").map(|_| true).unwrap_or(false)
}

/// Detect whether the terminal is Windows PowerShell (5.1 and below)
pub fn is_windows_powershell_legacy() -> bool {
    // Windows PowerShell 5.1 specific environment variable detection
    env::var("PSModulePath").is_ok()
        && env::var("PSVersionTable").is_err()
        && (env::var("PSHOME").is_ok() || env::var("PSExecutionPolicyPreference").is_ok())
}

/// Detect whether running inside Windows Terminal or ConEmu
pub fn is_windows_terminal_or_conemu() -> bool {
    // Windows Terminal - most reliable detection
    if env::var("WT_SESSION").is_ok() {
        return true;
    }

    // ConEmu with ANSI support
    if env::var("ConEmuANSI").is_ok() {
        return true;
    }

    // ANSICON - ANSI support layer
    if env::var("ANSICON").is_ok() {
        return true;
    }

    // TERM_PROGRAM - requires stricter validation
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        let term_program = term_program.to_lowercase();
        // Only confirm known modern terminal programs
        if [
            "vscode",
            "hyper",
            "terminus",
            "windowsterminal",
            "warp",
            "wt",
            "warpterminal",
        ]
        .contains(&term_program.as_str())
        {
            return true;
        }
    }

    false
}

/// Degradation decision function for Windows environments
fn should_degrade_on_windows(utf8_ok: bool, looks_modern: bool, vt_supported: bool) -> bool {
    // Virtual terminal processing confirmed, consider safe
    if vt_supported {
        return false;
    }

    // Case 1: Neither UTF-8 nor modern terminal -> definite degradation
    if !utf8_ok && !looks_modern {
        return true;
    }

    // Case 2: Windows PowerShell 5.1 not inside a modern terminal
    if is_windows_powershell_legacy() && !is_windows_terminal_or_conemu() {
        return true;
    }

    // Case 3: Legacy console environment detected (conhost)
    if is_traditional_console() {
        return true;
    }

    // Case 4: Non-UTF-8 environment, be conservative even if it looks modern
    if !utf8_ok && !is_very_modern_terminal() {
        return true;
    }

    // Case 5: Edge case - unable to clearly identify environment, use conservative strategy
    // Degrade if not an explicitly supported modern terminal and not a clearly legacy terminal
    if !is_very_modern_terminal() && !is_windows_terminal_or_conemu() && !is_powershell_core() {
        return true;
    }

    false
}

/// Detect whether the terminal is a legacy console (conhost)
fn is_traditional_console() -> bool {
    // Legacy consoles typically lack these modern environment variables
    env::var("WT_SESSION").is_err()
        && env::var("TERM_PROGRAM").is_err()
        && env::var("ConEmuANSI").is_err()
        && env::var("ANSICON").is_err()
        && (env::var("TERM").is_err() || env::var("TERM").unwrap_or_default().is_empty())
}

/// Detect whether the terminal is very modern (worth trying ANSI)
fn is_very_modern_terminal() -> bool {
    // Windows Terminal - most reliable detection
    if env::var("WT_SESSION").is_ok() {
        return true;
    }

    // VSCode terminal
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        if term_program.to_lowercase().contains("vscode") {
            return true;
        }
    }

    // Windows Terminal new version detection method
    if env::var("WT_PROFILE_ID").is_ok() {
        return true;
    }

    // Hyper terminal
    if let Ok(term) = env::var("TERM") {
        let term = term.to_lowercase();
        if term.contains("hyper") || term.contains("xterm-256color") {
            return true;
        }
    }

    // Detect other modern terminal environment variables
    if env::var("TERM_PROGRAM_VERSION").is_ok() {
        return true;
    }

    false
}

/// Detect whether the console has virtual terminal processing enabled (Windows only)
#[cfg(target_os = "windows")]
fn has_virtual_terminal_processing() -> bool {
    use winapi::um::consoleapi::GetConsoleMode;
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::STD_OUTPUT_HANDLE;
    use winapi::um::wincon::ENABLE_VIRTUAL_TERMINAL_PROCESSING;

    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return false;
        }

        let mut mode: u32 = 0;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return false;
        }

        mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0
    }
}

#[cfg(not(target_os = "windows"))]
fn has_virtual_terminal_processing() -> bool {
    true
}

fn bool_to_flag(v: bool) -> &'static str {
    if v { "1" } else { "0" }
}

fn detect_windows_utf8() -> bool {
    if !cfg!(target_os = "windows") {
        return true;
    }

    // Method 1: Check active code page
    if let Ok(output) = Command::new("cmd").args(["/C", "chcp"]).output() {
        if let Ok(text) = String::from_utf8(output.stdout) {
            // Look for "Active code page: 65001" or similar patterns
            if text.contains("65001") {
                return true;
            }
        }
    }

    // Method 2: Check system default output code page
    if let Ok(output) = Command::new("cmd").args(["/C", "echo %LANG%"]).output() {
        if let Ok(lang) = String::from_utf8(output.stdout) {
            let lang = lang.trim().to_lowercase();
            if lang.contains("utf-8") || lang.contains("65001") {
                return true;
            }
        }
    }

    // Method 3: Check system environment variables
    if let Ok(locale) = env::var("LC_ALL").or_else(|_| env::var("LANG")) {
        let locale = locale.to_lowercase();
        if locale.contains("utf-8") || locale.contains("65001") {
            return true;
        }
    }

    // Method 4: Check for Windows Terminal or other modern terminals
    if env::var("WT_SESSION")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        return true;
    }

    // Method 5: Check terminal program
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

    // Method 6: Check TERM variable
    if let Ok(term) = env::var("TERM") {
        let term = term.to_lowercase();
        if term.contains("xterm") || term.contains("screen") || term.contains("tmux") {
            return true;
        }
    }

    // Default conservative strategy
    false
}
