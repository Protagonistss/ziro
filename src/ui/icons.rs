//! Icon management module
//!
//! Provides cross-platform icon support: Unicode Emoji first, then narrow-width symbols, finally ASCII fallback.

use crate::platform::term::is_truthy_env;

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
    ///
    /// Relies on env vars set by `apply_profile_env()` during startup.
    /// No need for redundant terminal capability detection here.
    fn detect_mode() -> IconMode {
        if is_truthy_env("ZIRO_PLAIN") || is_truthy_env("ZIRO_ASCII_ICONS") {
            return IconMode::Ascii;
        }

        if is_truthy_env("ZIRO_NARROW") {
            return IconMode::Narrow;
        }

        IconMode::Unicode
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
