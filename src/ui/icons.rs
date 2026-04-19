//! Icon management module
//!
//! Provides cross-platform icon support: Unicode Emoji first, then narrow-width symbols, finally ASCII fallback.

use std::env;

#[derive(Clone, Copy, Debug)]
enum IconMode {
    Unicode,
    Narrow,
    Ascii,
}

/// Icon manager
pub struct Icons {
    mode: IconMode,
}

/// Three-tier icons (Unicode / Narrow / ASCII)
#[derive(Clone, Copy)]
pub struct IconGlyph {
    unicode: &'static str,
    narrow: &'static str,
    ascii: &'static str,
}

/// Predefined safe icons
pub struct SafeIcons;

impl SafeIcons {
    /// Success/complete mark
    pub const CHECK: IconGlyph = IconGlyph {
        unicode: "\u{2714}",
        narrow: "\u{2713}",
        ascii: "+",
    };

    /// Error/failure mark
    pub const CROSS: IconGlyph = IconGlyph {
        unicode: "\u{2716}",
        narrow: "\u{00D7}",
        ascii: "x",
    };

    /// Lightning/port related
    pub const LIGHTNING: IconGlyph = IconGlyph {
        unicode: "\u{26A1}",
        narrow: "*",
        ascii: "*",
    };

    /// Search/find
    pub const SEARCH: IconGlyph = IconGlyph {
        unicode: "\u{1F50D}",
        narrow: "?",
        ascii: "?",
    };

    /// Warning
    pub const WARNING: IconGlyph = IconGlyph {
        unicode: "\u{26A0}",
        narrow: "!",
        ascii: "!",
    };

    /// Fire/force kill
    pub const FIRE: IconGlyph = IconGlyph {
        unicode: "\u{1F525}",
        narrow: "!",
        ascii: "!",
    };

    /// Folder
    pub const FOLDER: IconGlyph = IconGlyph {
        unicode: "\u{1F4C2}",
        narrow: "[D]",
        ascii: "[D]",
    };

    /// File
    pub const FILE: IconGlyph = IconGlyph {
        unicode: "\u{1F4C4}",
        narrow: "[F]",
        ascii: "[F]",
    };

    /// Link
    pub const LINK: IconGlyph = IconGlyph {
        unicode: "\u{1F517}",
        narrow: "->",
        ascii: "->",
    };
}

impl Default for Icons {
    fn default() -> Self {
        Self::new()
    }
}

impl Icons {
    /// Create a new icon manager instance
    pub fn new() -> Self {
        let mode = Self::detect_mode();
        Self { mode }
    }

    /// Detect terminal/config to choose icon tier
    fn detect_mode() -> IconMode {
        // Explicit plain text mode: ASCII
        if is_truthy_env("ZIRO_PLAIN") {
            return IconMode::Ascii;
        }

        // Force ASCII
        if is_truthy_env("ZIRO_ASCII_ICONS") {
            return IconMode::Ascii;
        }

        // Force Unicode
        if is_truthy_env("ZIRO_UNICODE_ICONS") {
            return IconMode::Unicode;
        }

        // Force narrow-width symbols
        if is_truthy_env("ZIRO_NARROW") {
            return IconMode::Narrow;
        }

        // If not UTF-8/65001, prefer ASCII to avoid garbled output
        if is_likely_non_utf8() {
            return IconMode::Ascii;
        }

        // Default based on terminal capabilities
        if Self::detect_unicode_support() {
            IconMode::Unicode
        } else {
            IconMode::Ascii
        }
    }

    /// Detect whether terminal supports Unicode emoji
    fn detect_unicode_support() -> bool {
        // Check explicit locale settings first
        if let Ok(locale) = env::var("LC_ALL").or_else(|_| env::var("LANG"))
            && (locale.to_lowercase().contains("utf-8") || locale.contains("65001"))
        {
            return true;
        }

        // Check terminal type
        if let Ok(term) = env::var("TERM") {
            let term = term.to_lowercase();

            // Modern terminals with confirmed Unicode support
            if term.contains("xterm")
                || term.contains("screen")
                || term.contains("tmux")
                || term.contains("alacritty")
                || term.contains("kitty")
                || term.contains("iterm")
                || term.contains("gnome")
                || term.contains("konsole")
                || term.contains("rxvt")
                || term.contains("st")
            {
                return true;
            }

            // Windows-specific terminal types need more careful detection
            if cfg!(target_os = "windows") {
                if term.contains("cygwin") || term.contains("msys") || term.contains("mingw") {
                    // These terminals usually support Unicode
                    return true;
                } else if term.contains("win32")
                    || term.contains("conhost")
                    || term.contains("dumb")
                {
                    // Conservative: traditional Windows console may not support Unicode emoji
                    return false;
                }
            }
        }

        // Windows-specific detection
        if cfg!(target_os = "windows") {
            // Windows Terminal detection
            if let Ok(wt_session) = env::var("WT_SESSION") {
                return !wt_session.is_empty();
            }

            // Check terminal program
            if let Ok(term_program) = env::var("TERM_PROGRAM") {
                let term_program = term_program.to_lowercase();
                if [
                    "vscode",
                    "hyper",
                    "terminus",
                    "windowsterminal",
                    "wt",
                    "warp",
                    "warpterminal",
                ]
                .contains(&term_program.as_str())
                {
                    return true;
                }
            }

            // Check shell environment (Git Bash, WSL, etc.)
            if let Ok(shell) = env::var("SHELL") {
                if shell.contains("bash") || shell.contains("zsh") || shell.contains("fish") {
                    return true;
                }
            }

            // Check Windows Terminal install path
            if let Ok(program_files) = env::var("ProgramFiles") {
                let wt_path = std::path::Path::new(&program_files)
                    .join("WindowsApps")
                    .join("Microsoft.WindowsTerminal");
                if wt_path.exists() {
                    return true;
                }
            }

            // Check local app data for Windows Terminal
            if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
                let wt_path = std::path::Path::new(&local_app_data)
                    .join("Microsoft")
                    .join("WindowsApps");
                if wt_path.exists() && wt_path.join("Microsoft.WindowsTerminal").exists() {
                    return true;
                }
            }

            // Check enhanced terminal support
            if env::var("ConEmuANSI").is_ok() || env::var("ANSICON").is_ok() {
                return true;
            }

            // Default: modern Windows systems tend to support Unicode
            // unless a legacy console is explicitly detected
            if let Ok(term) = env::var("TERM") {
                if !term.is_empty() && !term.contains("win32") && !term.contains("conhost") {
                    return true;
                }
            }
        }

        // Default behavior for non-Windows systems
        #[cfg(not(target_os = "windows"))]
        {
            // Modern Unix/Linux systems almost always support Unicode
            true
        }

        #[cfg(target_os = "windows")]
        {
            // Windows default: if TERM is set, usually supports Unicode
            env::var("TERM").is_ok() && !env::var("TERM").unwrap_or_default().is_empty()
        }
    }

    pub fn check(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::CHECK, self.mode)
    }

    pub fn cross(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::CROSS, self.mode)
    }

    pub fn lightning(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::LIGHTNING, self.mode)
    }

    pub fn search(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::SEARCH, self.mode)
    }

    pub fn warning(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::WARNING, self.mode)
    }

    pub fn fire(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::FIRE, self.mode)
    }

    pub fn folder(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::FOLDER, self.mode)
    }

    pub fn file(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::FILE, self.mode)
    }

    pub fn link(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::LINK, self.mode)
    }
}

fn is_truthy_env(key: &str) -> bool {
    env::var(key)
        .map(|v| crate::platform::term::is_truthy(&v))
        .unwrap_or(false)
}

fn is_likely_non_utf8() -> bool {
    if cfg!(target_os = "windows") {
        // Windows Terminal or modern terminals usually support Unicode
        if env::var("WT_SESSION")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        {
            return false;
        }

        // Check terminal program
        if let Ok(term_program) = env::var("TERM_PROGRAM") {
            let term_program = term_program.to_lowercase();
            if [
                "vscode",
                "hyper",
                "terminus",
                "windowsterminal",
                "wt",
                "warp",
                "warpterminal",
            ]
            .contains(&term_program.as_str())
            {
                return false;
            }
        }

        // Consider available when LANG/LC_ALL contains utf-8
        let locale = env::var("LC_ALL")
            .or_else(|_| env::var("LANG"))
            .unwrap_or_default()
            .to_lowercase();
        if locale.contains("utf-8") || locale.contains("65001") {
            return false;
        }

        // Check TERM variable
        if let Ok(term) = env::var("TERM") {
            let term = term.to_lowercase();
            // Modern terminal types - more aggressive identification
            if term.contains("xterm")
                || term.contains("screen")
                || term.contains("tmux")
                || term.contains("alacritty")
                || term.contains("kitty")
                || term.contains("iterm")
                || term.contains("gnome")
                || term.contains("konsole")
            {
                return false;
            }
            // Traditional Windows console
            if term.contains("win32") || term.contains("conhost") || term.contains("dumb") {
                return true;
            }
        }

        // Check enhanced terminal support
        if env::var("ConEmuANSI").is_ok() || env::var("ANSICON").is_ok() {
            return false;
        }

        // Check if running in Git Bash, WSL, etc.
        if let Ok(shell) = env::var("SHELL") {
            if shell.contains("bash") || shell.contains("zsh") || shell.contains("fish") {
                return false;
            }
        }

        // Check Windows Terminal install path
        if let Ok(program_files) = env::var("ProgramFiles") {
            let wt_path = std::path::Path::new(&program_files)
                .join("WindowsApps")
                .join("Microsoft.WindowsTerminal");
            if wt_path.exists() {
                return false;
            }
        }

        // Improved fallback: only consider non-UTF-8 when legacy console is explicitly detected
        // All other cases (including empty TERM) lean towards Unicode support
        return false;
    }

    // Non-Windows: check if LANG/LC_ALL contains UTF-8
    let locale = env::var("LC_ALL")
        .or_else(|_| env::var("LANG"))
        .unwrap_or_default()
        .to_lowercase();

    // If no explicit locale info, conservatively assume UTF-8 support
    if locale.is_empty() {
        return false;
    }

    !locale.contains("utf-8")
}

/// Styled icon wrapper
pub struct StyledEmoji {
    glyph: IconGlyph,
    mode: IconMode,
}

impl StyledEmoji {
    fn new(glyph: IconGlyph, mode: IconMode) -> Self {
        Self { glyph, mode }
    }

    pub fn as_str(&self) -> &str {
        match self.mode {
            IconMode::Unicode => self.glyph.unicode,
            IconMode::Narrow => self.glyph.narrow,
            IconMode::Ascii => self.glyph.ascii,
        }
    }
}

impl std::fmt::Display for StyledEmoji {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Get cached icon manager instance
pub fn icons() -> &'static Icons {
    use std::sync::OnceLock;
    static INSTANCE: OnceLock<Icons> = OnceLock::new();
    INSTANCE.get_or_init(Icons::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_creation() {
        let icons = Icons::new();
        let check = icons.check();
        assert!(!check.as_str().is_empty());
    }
}
