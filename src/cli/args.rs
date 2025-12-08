use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Ziro - 跨平台端口管理工具
#[derive(Parser)]
#[command(name = "ziro")]
#[command(about = "查找和终止占用端口的进程", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(disable_version_flag = true)]
pub struct Cli {
    /// 显示版本信息
    #[arg(short = 'v', long = "version")]
    pub version: bool,

    /// 强制使用 ASCII 图标（等效于设置 ZIRO_ASCII_ICONS=1）
    #[arg(long = "ascii")]
    pub ascii: bool,

    /// 强制关闭颜色（等效于设置 ZIRO_NO_COLOR=1）
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// 使用窄字符符号（尽量避免宽字符乱码，等效于 ZIRO_NARROW=1）
    #[arg(long = "narrow")]
    pub narrow: bool,

    /// 纯文本模式（ASCII + 无色，等效于 ZIRO_PLAIN=1）
    #[arg(long = "plain")]
    pub plain: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 查找占用指定端口的进程
    Find {
        /// 要查找的端口号（可以指定多个）
        ports: Vec<u16>,
    },
    /// 终止占用指定端口的进程
    Kill {
        /// 要终止的端口号（可以指定多个）
        ports: Vec<u16>,
        /// 强制终止（不询问确认）
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// 列出所有端口占用情况
    List,
    /// 删除文件或目录（支持递归删除）
    Remove {
        /// 要删除的文件或目录路径（可以指定多个）
        paths: Vec<PathBuf>,
        /// 强制删除（不询问确认）
        #[arg(short = 'f', long = "force")]
        force: bool,
        /// 递归删除目录及其内容
        #[arg(short = 'r', long = "recursive")]
        recursive: bool,
        /// 显示将要删除的内容但不实际删除
        #[arg(short = 'n', long = "dry-run")]
        dry_run: bool,
        /// 显示详细的删除过程信息
        #[arg(long = "verbose")]
        verbose: bool,
        /// 忽略占用提示，直接尝试删除
        #[arg(long = "anyway")]
        anyway: bool,
    },
    /// 实时查看进程内存占用（类似 top）
    Top {
        /// 刷新间隔（秒）
        #[arg(long = "interval", default_value_t = 1.0)]
        interval: f32,
        /// 显示的进程数量
        #[arg(long = "limit", default_value_t = 20)]
        limit: usize,
        /// 同时显示 CPU 占用
        #[arg(long = "cpu")]
        cpu: bool,
        /// 显示进程的命令行
        #[arg(long = "cmd")]
        cmd: bool,
        /// 只输出一次，不持续刷新
        #[arg(long = "once")]
        once: bool,
    },
}
