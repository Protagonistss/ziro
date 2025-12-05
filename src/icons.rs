//! 图标管理模块
//!
//! 提供跨平台的图标支持，自动检测终端能力并选择合适的图标
//! 在支持 Unicode 的现代终端中使用 emoji，在传统终端中回退到 ASCII 字符

use console::{Emoji, style};
use std::env;

/// 图标管理器
pub struct Icons {
    /// 是否使用 Unicode emoji
    use_unicode: bool,
}

/// 预定义的安全图标
pub struct SafeIcons;

impl SafeIcons {
    /// 成功/完成标记
    pub const CHECK: Emoji<'static, 'static> = Emoji("✓", "+");

    /// 错误/失败标记
    pub const CROSS: Emoji<'static, 'static> = Emoji("✗", "x");

    /// 闪电/端口相关
    pub const LIGHTNING: Emoji<'static, 'static> = Emoji("⚡", "*");

    /// 搜索/查找
    pub const SEARCH: Emoji<'static, 'static> = Emoji("🔍", "?");

    /// 警告
    pub const WARNING: Emoji<'static, 'static> = Emoji("⚠️", "!");

    /// 火/强制终止
    pub const FIRE: Emoji<'static, 'static> = Emoji("🔥", "!");

    /// 文件夹
    pub const FOLDER: Emoji<'static, 'static> = Emoji("📁", "[D]");

    /// 文件
    pub const FILE: Emoji<'static, 'static> = Emoji("📄", "[F]");

    /// 链接
    pub const LINK: Emoji<'static, 'static> = Emoji("🔗", "->");
}

impl Default for Icons {
    fn default() -> Self {
        Self::new()
    }
}

impl Icons {
    /// 创建新的图标管理器实例
    pub fn new() -> Self {
        let use_unicode = Self::detect_unicode_support();
        Self { use_unicode }
    }

    /// 检测终端是否支持 Unicode emoji
    fn detect_unicode_support() -> bool {
        // 检查环境变量
        if let Ok(force_ascii) = env::var("ZIRO_ASCII_ICONS") {
            return force_ascii != "1" && force_ascii.to_lowercase() != "true";
        }

        if let Ok(force_unicode) = env::var("ZIRO_UNICODE_ICONS") {
            return force_unicode == "1" || force_unicode.to_lowercase() == "true";
        }

        // 检查终端类型
        if let Ok(term) = env::var("TERM") {
            // 这些终端通常支持 Unicode
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
            // Windows Terminal、现代 PowerShell 通常支持 Unicode
            if let Ok(wt_session) = env::var("WT_SESSION") {
                return !wt_session.is_empty(); // Windows Terminal
            }

            // 检查是否在 Windows Terminal 中运行
            if let Ok(program_files) = env::var("ProgramFiles") {
                let wt_path = std::path::Path::new(&program_files)
                    .join("WindowsApps")
                    .join("Microsoft.WindowsTerminal");
                if wt_path.exists() {
                    return true;
                }
            }
        }

        // 默认在现代系统上启用 Unicode，传统系统上禁用
        #[cfg(target_os = "windows")]
        {
            // Windows 上保守一些，默认使用 ASCII
            false
        }

        #[cfg(not(target_os = "windows"))]
        {
            // Linux/macOS 上默认启用 Unicode
            true
        }
    }

    /// 获取成功标记
    pub fn check(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::CHECK, self.use_unicode)
    }

    /// 获取错误标记
    pub fn cross(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::CROSS, self.use_unicode)
    }

    /// 获取闪电图标
    pub fn lightning(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::LIGHTNING, self.use_unicode)
    }

    /// 获取搜索图标
    pub fn search(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::SEARCH, self.use_unicode)
    }

    /// 获取警告图标
    pub fn warning(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::WARNING, self.use_unicode)
    }

    /// 获取火图标
    pub fn fire(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::FIRE, self.use_unicode)
    }

    /// 获取文件夹图标
    pub fn folder(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::FOLDER, self.use_unicode)
    }

    /// 获取文件图标
    pub fn file(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::FILE, self.use_unicode)
    }

    /// 获取链接图标
    pub fn link(&self) -> StyledEmoji {
        StyledEmoji::new(SafeIcons::LINK, self.use_unicode)
    }
}

/// 带样式的 emoji 包装器
pub struct StyledEmoji {
    emoji: Emoji<'static, 'static>,
    use_unicode: bool,
}

impl StyledEmoji {
    fn new(emoji: Emoji<'static, 'static>, use_unicode: bool) -> Self {
        Self { emoji, use_unicode }
    }

    /// 获取图标字符串
    pub fn as_str(&self) -> &str {
        if self.use_unicode {
            self.emoji.0
        } else {
            self.emoji.1
        }
    }

    /// 获取绿色样式的成功图标
    pub fn green(&self) -> String {
        style(self.as_str()).green().to_string()
    }

    /// 获取红色样式的错误图标
    pub fn red(&self) -> String {
        style(self.as_str()).red().to_string()
    }

    /// 蓝色样式
    pub fn blue(&self) -> String {
        style(self.as_str()).blue().to_string()
    }

    /// 青色样式
    pub fn cyan(&self) -> String {
        style(self.as_str()).cyan().to_string()
    }

    /// 黄色样式
    #[allow(dead_code)]
    pub fn yellow(&self) -> String {
        style(self.as_str()).yellow().to_string()
    }

    /// 紫色样式
    #[allow(dead_code)]
    pub fn magenta(&self) -> String {
        style(self.as_str()).magenta().to_string()
    }

    /// 粗体样式
    #[allow(dead_code)]
    pub fn bold(&self) -> String {
        style(self.as_str()).bold().to_string()
    }

    /// 作为普通字符串（用于格式化）
    #[allow(dead_code)]
    pub fn display(&self) -> &str {
        self.as_str()
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
