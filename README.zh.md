# Ziro

<div align="center">

一个快速、跨平台的端口管理工具

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/ziro.svg)](https://crates.io/crates/ziro)
[![npm](https://img.shields.io/npm/v/@ithinku/ziro.svg)](https://www.npmjs.com/package/@ithinku/ziro)

[English](README.en.md) | [简体中文](README.zh.md)

</div>

## 简介

Ziro 是一个强大的命令行工具，用于快速查找和管理占用端口的进程。支持 Windows、Linux 和 macOS 平台。

### 核心特性

- 🔍 **快速查找** - 即时查找占用指定端口的进程
- 🎯 **批量终止** - 支持同时终止多个端口的进程
- 📊 **详细信息** - 显示进程 PID、名称、命令、CPU 和内存使用情况
- 🎨 **美观界面** - 彩色输出和表格展示，提供更好的视觉体验
- 💬 **交互式选择** - 终止进程前可交互式选择和确认
- 🌍 **跨平台** - 支持 Windows、Linux 和 macOS

## 安装

### 使用 Cargo（Rust 用户）

```bash
cargo install ziro
```

### 使用 npm（Node.js 用户）

```bash
npm install -g @ithinku/ziro
```

或使用其他包管理器：

```bash
# 使用 yarn
yarn global add @ithinku/ziro

# 使用 pnpm
pnpm add -g @ithinku/ziro
```

## 使用方法

### 查找占用端口的进程

```bash
# 查找占用 8080 端口的进程
ziro find 8080
```

输出示例：
```
找到占用端口的进程：
  端口: 8080
  PID: 12345
  名称: node
  命令: node server.js
  CPU: 2.3%
  内存: 128 MB
```

### 终止占用端口的进程

```bash
# 终止占用 8080 端口的进程
ziro kill 8080

# 终止多个端口的进程
ziro kill 8080 3000 5000
```

程序会显示找到的所有进程，让你交互式地选择要终止的进程，并在终止前进行确认。

### 列出所有端口占用情况

```bash
ziro list
```

输出示例：
```
当前端口占用情况：
╭──────┬───────┬──────────┬─────────────────────────┬───────┬────────╮
│ 端口 │  PID  │   名称   │          命令           │  CPU  │  内存  │
├──────┼───────┼──────────┼─────────────────────────┼───────┼────────┤
│ 3000 │ 12345 │ node     │ node app.js             │ 1.2%  │ 95 MB  │
│ 8080 │ 23456 │ python   │ python -m http.server   │ 0.5%  │ 45 MB  │
│ 5432 │ 34567 │ postgres │ /usr/bin/postgres       │ 3.1%  │ 256 MB │
╰──────┴───────┴──────────┴─────────────────────────┴───────┴────────╯
```

### 查看文件/目录占用

```bash
# 查看单个文件
ziro who C:\path\file.txt

# 查看多个路径
ziro who .\logs .\data\app.db
```

输出会显示是否被占用，并在可用时列出相关进程。

## 命令参考

```
Ziro - 跨平台端口管理工具

使用方法:
  ziro <COMMAND>

命令:
  find <PORT>          查找占用指定端口的进程
  kill <PORT>...       终止占用指定端口的进程（可指定多个）
  list                 列出所有端口占用情况
  who <PATH>...        查找占用指定文件或目录的进程
  help                 显示帮助信息

选项:
  -h, --help           显示帮助信息
  -V, --version        显示版本信息
```

## 平台支持

| 操作系统 | 架构 | 支持状态 |
|---------|------|---------|
| Windows | x64  | ✅ 完全支持 |
| Linux   | x64  | ✅ 完全支持 |
| Linux   | arm64| ✅ 完全支持 |
| macOS   | x64  | ✅ 完全支持 |
| macOS   | arm64| ✅ 完全支持 |

## 技术栈

- **核心语言**: Rust
- **命令行解析**: clap
- **系统信息**: sysinfo
- **交互界面**: inquire
- **彩色输出**: colored

## 开发

### 构建项目

```bash
# 克隆仓库
git clone https://github.com/Protagonistss/ziro.git
cd ziro

# 构建
cargo build --release

# 运行
cargo run -- find 8080
```

### 运行测试

```bash
cargo test
```

### 代码格式化

```bash
cargo fmt
```

### 代码检查

```bash
cargo clippy
```

## 贡献

欢迎贡献！请随意提交 Issue 或 Pull Request。

### 贡献指南

1. Fork 本项目
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

本项目基于 [MIT License](LICENSE) 开源。

## 致谢

感谢所有贡献者和使用者的支持！

## 相关项目

- [fkill](https://github.com/sindresorhus/fkill) - Node.js 版本的进程终止工具
- [lsof](https://github.com/lsof-org/lsof) - Unix 系统的文件和网络连接查看工具

## 更新日志

查看 [CHANGELOG.md](CHANGELOG.md) 了解版本历史。

---

<div align="center">
  
**如果这个项目对你有帮助，请给一个 ⭐️**

Made with ❤️ by [huangshan](https://github.com/Protagonistss)

</div>

