use crate::platform::term;
use crate::ui::icons;
use crate::ui::icons::StyledEmoji;
use colored::{Color, Colorize};
use std::sync::OnceLock;

static THEME: OnceLock<Theme> = OnceLock::new();

#[derive(Clone)]
pub struct Theme {
    use_color: bool,
}

impl Theme {
    pub fn new() -> Self {
        THEME.get().cloned().unwrap_or_else(Self::build)
    }

    fn build() -> Self {
        Self {
            use_color: Self::detect_color_support(),
        }
    }

    /// Detect whether color is enabled
    fn detect_color_support() -> bool {
        let plain = std::env::var("ZIRO_PLAIN")
            .map(|v| term::is_truthy(&v))
            .unwrap_or(false);
        let no_color = std::env::var("ZIRO_NO_COLOR")
            .map(|v| term::is_truthy(&v))
            .unwrap_or(false)
            || std::env::var("NO_COLOR")
                .map(|v| v.is_empty() || term::is_truthy(&v))
                .unwrap_or(false);
        !plain && !no_color
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
        self.paint_icon(icons::icons().folder(), Color::Cyan)
    }

    pub fn icon_file(&self) -> String {
        self.paint_icon(icons::icons().file(), Color::Blue)
    }

    pub fn icon_link(&self) -> String {
        self.paint_icon(icons::icons().link(), Color::Magenta)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}
