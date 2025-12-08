//! 图标管理模块
//!
//! 提供跨平台的图标支持：优先 Unicode Emoji，其次窄字符符号，最后 ASCII 回退。

use std::env;

#[derive(Clone, Copy, Debug)]
enum IconMode {
    Unicode,
    Narrow,
    Ascii,
}

/// 图标管理器
pub struct Icons {
    mode: IconMode,
}

/// 三档图标（Unicode / 窄字符 / ASCII）
#[derive(Clone, Copy)]
pub struct IconGlyph {
    unicode: &'static str,
    narrow: &'static str,
    ascii: &'static str,
}

/// 预定义的安全图标
pub struct SafeIcons;

impl SafeIcons {
    /// 成功/完成标记
    pub const CHECK: IconGlyph = IconGlyph {
        unicode: "\u{2714}",
        narrow: "\u{2713}",
        ascii: "+",
    };

    /// 错误/失败标记
    pub const CROSS: IconGlyph = IconGlyph {
        unicode: "\u{2716}",
        narrow: "\u{00D7}",
        ascii: "x",
    };

    /// 闪电/端口相关
    pub const LIGHTNING: IconGlyph = IconGlyph {
        unicode: "\u{26A1}",
        narrow: "*",
        ascii: "*",
    };

    /// 搜索/查找
    pub const SEARCH: IconGlyph = IconGlyph {
        unicode: "\u{1F50D}",
        narrow: "?",
        ascii: "?",
    };

    /// 警告
    pub const WARNING: IconGlyph = IconGlyph {
        unicode: "\u{26A0}",
        narrow: "!",
        ascii: "!",
    };

    /// 火/强制终止
    pub const FIRE: IconGlyph = IconGlyph {
        unicode: "\u{1F525}",
        narrow: "!",
        ascii: "!",
    };

    /// 文件夹
    pub const FOLDER: IconGlyph = IconGlyph {
        unicode: "\u{1F4C2}",
        narrow: "[D]",
        ascii: "[D]",
    };

    /// 文件
    pub const FILE: IconGlyph = IconGlyph {
        unicode: "\u{1F4C4}",
        narrow: "[F]",
        ascii: "[F]",
    };

    /// 链接
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
    /// 创建新的图标管理器实例
    pub fn new() -> Self {
        let mode = Self::detect_mode();
        Self { mode }
    }

    /// 检测终端/配置选择哪个图标档位
    fn detect_mode() -> IconMode {
        // 显式纯文本模式：ASCII
        if is_truthy_env("ZIRO_PLAIN") {
            return IconMode::Ascii;
        }

        // 强制 ASCII
        if is_truthy_env("ZIRO_ASCII_ICONS") {
            return IconMode::Ascii;
        }

        // 强制 Unicode
        if is_truthy_env("ZIRO_UNICODE_ICONS") {
            return IconMode::Unicode;
        }

        // 强制窄字符（单宽符号）
        if is_truthy_env("ZIRO_NARROW") {
            return IconMode::Narrow;
        }

        // 如果不是 UTF-8/65001，优先用 ASCII，避免乱码
        if is_likely_non_utf8() {
            return IconMode::Ascii;
        }

        // 基于终端能力的默认选择
        if Self::detect_unicode_support() {
            IconMode::Unicode
        } else {
            IconMode::Ascii
        }
    }

    /// 检测终端是否支持 Unicode emoji
    fn detect_unicode_support() -> bool {
        // 首先检查明确的语言环境设置
        if let Ok(locale) = env::var("LC_ALL").or_else(|_| env::var("LANG"))
            && (locale.to_lowercase().contains("utf-8") || locale.contains("65001"))
        {
            return true;
        }

        // 检查终端类型
        if let Ok(term) = env::var("TERM") {
            let term = term.to_lowercase();

            // 明确支持 Unicode 的现代终端
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

            // 对于 Windows 特有的终端类型，需要更仔细的判断
            if cfg!(target_os = "windows") {
                if term.contains("cygwin") || term.contains("msys") || term.contains("mingw") {
                    // 这些终端通常支持 Unicode
                    return true;
                } else if term.contains("win32")
                    || term.contains("conhost")
                    || term.contains("dumb")
                {
                    // 保守策略：传统 Windows 控制台可能不支持 Unicode emoji
                    return false;
                }
            }
        }

        // Windows 特定检测
        if cfg!(target_os = "windows") {
            // Windows Terminal 检测
            if let Ok(wt_session) = env::var("WT_SESSION") {
                return !wt_session.is_empty();
            }

            // 检查终端程序
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

            // 检查 Shell 环境（Git Bash, WSL 等）
            if let Ok(shell) = env::var("SHELL") {
                if shell.contains("bash") || shell.contains("zsh") || shell.contains("fish") {
                    return true;
                }
            }

            // 检查 Windows Terminal 安装路径
            if let Ok(program_files) = env::var("ProgramFiles") {
                let wt_path = std::path::Path::new(&program_files)
                    .join("WindowsApps")
                    .join("Microsoft.WindowsTerminal");
                if wt_path.exists() {
                    return true;
                }
            }

            // 检查本地应用数据中的 Windows Terminal
            if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
                let wt_path = std::path::Path::new(&local_app_data)
                    .join("Microsoft")
                    .join("WindowsApps");
                if wt_path.exists() && wt_path.join("Microsoft.WindowsTerminal").exists() {
                    return true;
                }
            }

            // 检查增强终端支持
            if env::var("ConEmuANSI").is_ok() || env::var("ANSICON").is_ok() {
                return true;
            }

            // 默认情况下，现代 Windows 系统倾向于支持 Unicode
            // 除非明确检测到传统控制台
            if let Ok(term) = env::var("TERM") {
                if !term.is_empty() && !term.contains("win32") && !term.contains("conhost") {
                    return true;
                }
            }
        }

        // 非 Windows 系统的默认行为
        #[cfg(not(target_os = "windows"))]
        {
            // 现代 Unix/Linux 系统几乎都支持 Unicode
            true
        }

        #[cfg(target_os = "windows")]
        {
            // Windows 的默认行为：如果有 TERM 变量，通常支持 Unicode
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
    if let Ok(v) = env::var(key) {
        let v = v.to_lowercase();
        return matches!(v.as_str(), "1" | "true" | "yes" | "on");
    }
    false
}

fn is_likely_non_utf8() -> bool {
    if cfg!(target_os = "windows") {
        // Windows Terminal 或现代终端通常支持 Unicode
        if env::var("WT_SESSION")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        {
            return false;
        }

        // 检查终端程序
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

        // LANG/LC_ALL 包含 utf-8 时认为可用
        let locale = env::var("LC_ALL")
            .or_else(|_| env::var("LANG"))
            .unwrap_or_default()
            .to_lowercase();
        if locale.contains("utf-8") || locale.contains("65001") {
            return false;
        }

        // 检查 TERM 变量
        if let Ok(term) = env::var("TERM") {
            let term = term.to_lowercase();
            // 现代终端类型 - 更积极的识别
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
            // 传统 Windows 控制台
            if term.contains("win32") || term.contains("conhost") || term.contains("dumb") {
                return true;
            }
        }

        // 检查增强终端支持
        if env::var("ConEmuANSI").is_ok() || env::var("ANSICON").is_ok() {
            return false;
        }

        // 检查是否在 Git Bash、WSL 等环境中
        if let Ok(shell) = env::var("SHELL") {
            if shell.contains("bash") || shell.contains("zsh") || shell.contains("fish") {
                return false;
            }
        }

        // 检查 Windows Terminal 安装路径
        if let Ok(program_files) = env::var("ProgramFiles") {
            let wt_path = std::path::Path::new(&program_files)
                .join("WindowsApps")
                .join("Microsoft.WindowsTerminal");
            if wt_path.exists() {
                return false;
            }
        }

        // 改进的回退策略：只有在明确检测到传统控制台时才认为是非 UTF-8
        // 其他情况（包括空 TERM 变量）都倾向于支持 Unicode
        return false;
    }

    // 非 Windows：检查 LANG/LC_ALL 是否包含 UTF-8
    let locale = env::var("LC_ALL")
        .or_else(|_| env::var("LANG"))
        .unwrap_or_default()
        .to_lowercase();

    // 如果没有明确的 locale 信息，保守地认为支持 UTF-8
    if locale.is_empty() {
        return false;
    }

    !locale.contains("utf-8")
}

/// 带样式的图标包装器
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

/// 获取图标管理器实例
pub fn icons() -> Icons {
    Icons::new()
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
