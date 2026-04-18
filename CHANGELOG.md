## v0.0.22 (2026-04-18)

Changes since v0.0.21:

- Merge pull request #3 from Protagonistss/dev (0ff0265)
- fix: remove dead code for non-Windows stubs and fix clippy single_match (d730d5b)
- fix(lock): use compile-time cfg instead of runtime cfg! for platform-specific functions (6ff0448)
- style: apply cargo fmt formatting (b3ffda6)
- fix: resolve cross-platform compilation errors for non-Windows (caf0c6b)
- fix(ci): add rustfmt and clippy to rust-toolchain.toml (2a062eb)
- Merge pull request #2 from Protagonistss/dev (f3d08e0)
- chore: translate package description to English (50d8352)
- chore(ci): use rust-toolchain.toml for version pinning (def72c7)
- feat(ci): rewrite release workflow with workflow_dispatch and checksums (6d24a10)
- chore: pin Rust toolchain to 1.88.0 (46d10b5)
- chore(release): add optimized release profile (741a55b)
- Merge pull request #1 from Protagonistss/dev (1fa8803)
- refactor: Remove 功能 7 项优化 + 全项目中文替换为英文 (e74273f)
- fix(fs): 保留原始 IO 错误链以支持重试检测 (f9c4fa6)
- feat(fs): 添加删除重试机制，指数退避自动重试占用文件 (26da89d)
- fix(lock): 移除 PowerShell 回退中的 wmic 调用 (9be98c3)
- feat(lock): 使用 RestartManager API 检测文件占用进程 (d76f218)
- chore(deps): 添加 windows-sys RestartManager feature (73341db)
- docs: 添加 Windows 文件删除占用问题改进设计文档 (4089691)
- refactor: 代码优化 - 消除重复、重构模块、简化函数 (506de64)
- feat(fs): 集成文件占用检查功能到 remove 命令 (0f3adaf)
- feat(fs): 添加文件/目录占用检查功能 (9decf87)

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

