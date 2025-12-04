use clap::{Parser, Subcommand};

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
    },
    /// 列出所有端口占用情况
    List,
}
