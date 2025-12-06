//! 图标管理模块
//!
//! 提供跨平台的图标支持：优先 Unicode Emoji，其次窄字符符号，最后 ASCII 回退。

use std::env;

#[derive(Clone, Copy)]
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
        // 检查终端类型
        if let Ok(term) = env::var("TERM") {
            if term.contains("xterm")
                || term.contains("screen")
                || term.contains("tmux")
                || term.contains("alacritty")
                || term.contains("kitty")
            {
                return true;
            }
        }

        // Windows 终端检测
        if cfg!(target_os = "windows") {
            if let Ok(wt_session) = env::var("WT_SESSION") {
                return !wt_session.is_empty();
            }

            if let Ok(program_files) = env::var("ProgramFiles") {
                let wt_path = std::path::Path::new(&program_files)
                    .join("WindowsApps")
                    .join("Microsoft.WindowsTerminal");
                if wt_path.exists() {
                    return true;
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            false
        }

        #[cfg(not(target_os = "windows"))]
        {
            true
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
        // Windows Terminal 或 VSCode 终端通常支持 Unicode
        if env::var("WT_SESSION")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        {
            return false;
        }

        // LANG/LC_ALL 包含 utf-8 时认为可用
        let locale = env::var("LC_ALL")
            .or_else(|_| env::var("LANG"))
            .unwrap_or_default()
            .to_lowercase();
        if locale.contains("utf-8") || locale.contains("65001") {
            return false;
        }

        // 保守回退
        return true;
    }

    // 非 Windows：检查 LANG/LC_ALL 是否包含 UTF-8
    let locale = env::var("LC_ALL")
        .or_else(|_| env::var("LANG"))
        .unwrap_or_default()
        .to_lowercase();
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
