# 更新日志

所有重要的项目更改都将记录在此文件中。

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范。

## [未发布]

### 新增
- 初始版本发布

## [0.0.1] - 2025-12-03

### 新增
- ✨ 实现跨平台端口查找功能
  - 支持 Windows、Linux 和 macOS
  - 使用 `netstat`、`lsof` 和 `/proc/net` 获取端口信息
- ✨ 实现进程终止功能
  - 支持批量终止多个端口的进程
  - 交互式选择要终止的进程
  - 确认提示防止误操作
- ✨ 实现端口列表功能
  - 表格形式展示所有端口占用情况
  - 显示进程详细信息（PID、名称、命令、CPU、内存）
- 🎨 美化命令行输出
  - 彩色输出提升可读性
  - 表格展示更加直观
  - 友好的错误提示
- 📦 支持多种安装方式
  - Cargo 安装：`cargo install ziro`
  - NPM 安装：`npm install -g ziro`
- 🔧 完善项目配置
  - 添加 CI/CD 自动化流程
  - 多平台构建和发布
  - 版本号同步管理

### 技术栈
- Rust 2024 edition
- clap 4.5 - 命令行参数解析
- sysinfo 0.31 - 系统信息获取
- inquire 0.7 - 交互式命令行界面
- tabled 0.15 - 表格展示
- colored 2.1 - 彩色输出
- anyhow 1.0 - 错误处理

---

## 版本说明

### 版本格式
- **主版本号 (MAJOR)**: 不兼容的 API 变更
- **次版本号 (MINOR)**: 向下兼容的功能新增
- **修订号 (PATCH)**: 向下兼容的问题修正

### 变更类型
- `新增` - 新功能
- `变更` - 现有功能的变更
- `废弃` - 即将移除的功能
- `移除` - 已移除的功能
- `修复` - Bug 修复
- `安全` - 安全性修复

[未发布]: https://github.com/Protagonistss/ziro/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/Protagonistss/ziro/releases/tag/v0.0.1

