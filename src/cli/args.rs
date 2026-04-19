use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Ziro - Cross-platform port management tool
#[derive(Parser)]
#[command(name = "ziro")]
#[command(about = "Cross-platform port and process management tool", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// Force ASCII icons (equivalent to ZIRO_ASCII_ICONS=1)
    #[arg(long = "ascii")]
    pub ascii: bool,

    /// Disable colors (equivalent to ZIRO_NO_COLOR=1)
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// Use narrow-width symbols (equivalent to ZIRO_NARROW=1)
    #[arg(long = "narrow")]
    pub narrow: bool,

    /// Plain text mode: ASCII + no color (equivalent to ZIRO_PLAIN=1)
    #[arg(long = "plain")]
    pub plain: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Find processes occupying specified ports
    Find {
        /// Port numbers to find (multiple allowed)
        ports: Vec<u16>,
    },
    /// Kill processes occupying specified ports
    Kill {
        /// Port numbers to kill (multiple allowed)
        ports: Vec<u16>,
        /// Force kill without confirmation
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// List all port usage
    List,
    /// Find processes locking specified files or directories
    Who {
        /// File or directory paths to check (multiple allowed)
        paths: Vec<PathBuf>,
    },
    /// Remove files or directories (supports recursive deletion)
    Remove {
        /// File or directory paths to remove (multiple allowed)
        paths: Vec<PathBuf>,
        /// Force removal without confirmation
        #[arg(short = 'f', long = "force")]
        force: bool,
        /// Recursively remove directories and their contents
        #[arg(short = 'r', long = "recursive")]
        recursive: bool,
        /// Show what would be deleted without actually deleting
        #[arg(short = 'n', long = "dry-run")]
        dry_run: bool,
        /// Show detailed deletion progress
        #[arg(short = 'V', long = "verbose")]
        verbose: bool,
        /// Force kill processes locking the files, then delete
        #[arg(long = "anyway", visible_alias = "kill-lockers")]
        anyway: bool,
    },
    /// Monitor process memory usage in real time (like top)
    Top {
        /// Refresh interval in seconds
        #[arg(long = "interval", default_value_t = 1.0)]
        interval: f32,
        /// Number of processes to display
        #[arg(long = "limit", default_value_t = 20)]
        limit: usize,
        /// Show CPU usage alongside memory
        #[arg(long = "cpu")]
        cpu: bool,
        /// Show process command lines
        #[arg(long = "cmd")]
        cmd: bool,
        /// Output once without continuous refresh
        #[arg(long = "once")]
        once: bool,
    },
}
