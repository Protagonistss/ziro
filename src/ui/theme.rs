use crate::ui::icons;
use crate::ui::icons::StyledEmoji;
use colored::{Color, Colorize};
use std::env;

/// 统一的颜色与图标主题
pub struct Theme {
    use_color: bool,
}

impl Theme {
    pub fn new() -> Self {
        Self {
            use_color: Self::detect_color_support(),
        }
    }

    /// 检测是否启用颜色
    fn detect_color_support() -> bool {
        if let Ok(value) = env::var("ZIRO_PLAIN") {
            if Self::is_truthy(&value) {
                return false;
            }
        }

        if let Ok(value) = env::var("ZIRO_NO_COLOR") {
            if Self::is_truthy(&value) {
                return false;
            }
        }

        // 兼容通用的 NO_COLOR 约定
        if let Ok(value) = env::var("NO_COLOR") {
            if value.is_empty() || Self::is_truthy(&value) {
                return false;
            }
        }

        true
    }

    fn is_truthy(value: &str) -> bool {
        matches!(value.to_lowercase().as_str(), "1" | "true" | "yes" | "on")
    }

    fn paint(&self, text: impl AsRef<str>, color: Color, bold: bool) -> String {
        let content = text.as_ref();
        if !self.use_color {
            return content.to_string();
        }

        let styled = content.color(color);
        if bold {
            styled.bold().to_string()
        } else {
            styled.to_string()
        }
    }

    fn paint_icon(&self, icon: StyledEmoji, color: Color) -> String {
        let base = icon.to_string();
        if self.use_color {
            base.color(color).to_string()
        } else {
            base
        }
    }

    pub fn title(&self, text: impl AsRef<str>) -> String {
        self.paint(text, Color::Cyan, true)
    }

    pub fn success(&self, text: impl AsRef<str>) -> String {
        self.paint(text, Color::Green, false)
    }

    pub fn error(&self, text: impl AsRef<str>) -> String {
        self.paint(text, Color::Red, false)
    }

    pub fn error_bold(&self, text: impl AsRef<str>) -> String {
        self.paint(text, Color::Red, true)
    }

    pub fn warn(&self, text: impl AsRef<str>) -> String {
        self.paint(text, Color::Yellow, false)
    }

    pub fn info(&self, text: impl AsRef<str>) -> String {
        self.paint(text, Color::Cyan, false)
    }

    pub fn info_bold(&self, text: impl AsRef<str>) -> String {
        self.paint(text, Color::Cyan, true)
    }

    pub fn accent(&self, text: impl AsRef<str>) -> String {
        self.paint(text, Color::Magenta, false)
    }

    pub fn blue(&self, text: impl AsRef<str>) -> String {
        self.paint(text, Color::Blue, false)
    }

    pub fn muted(&self, text: impl AsRef<str>) -> String {
        self.paint(text, Color::BrightBlack, false)
    }

    pub fn highlight(&self, text: impl AsRef<str>) -> String {
        self.paint(text, Color::Yellow, true)
    }

    pub fn icon_success(&self) -> String {
        self.paint_icon(icons::icons().check(), Color::Green)
    }

    pub fn icon_error(&self) -> String {
        self.paint_icon(icons::icons().cross(), Color::Red)
    }

    pub fn icon_lightning(&self) -> String {
        self.paint_icon(icons::icons().lightning(), Color::Cyan)
    }

    pub fn icon_search(&self) -> String {
        self.paint_icon(icons::icons().search(), Color::Blue)
    }

    pub fn icon_warning(&self) -> String {
        self.paint_icon(icons::icons().warning(), Color::Red)
    }

    pub fn icon_fire(&self) -> String {
        self.paint_icon(icons::icons().fire(), Color::Red)
    }

    pub fn icon_folder(&self) -> String {
        icons::icons().folder().to_string()
    }

    pub fn icon_file(&self) -> String {
        icons::icons().file().to_string()
    }

    pub fn icon_link(&self) -> String {
        icons::icons().link().to_string()
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}
