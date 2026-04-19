# 开发指南

## 技术栈

- **核心语言**: Rust
- **命令行解析**: clap
- **系统信息**: sysinfo
- **交互界面**: inquire
- **彩色输出**: colored

## 项目结构与模块组织
- Rust 核心：入口在 `src/bin/ziro.rs`；`src/cli/`（`args.rs` 参数定义，`handlers.rs` 命令分发）；`src/core/`（`port/` 端口扫描，`process/` 进程终止，`fs_ops/` 文件删除与安全校验，`top/` 监控）；`src/platform/`（`term.rs` 终端/环境配置，`encoding.rs` Windows UTF-8 初始化）；`src/ui/`（`render.rs` 输出、交互，`theme.rs` 颜色，`icons.rs` 图标）。
- Node 分发层：`bin/ziro.js` 作为 npm 启动代理，`scripts/install.js`+`detect-platform.js` 下载发布产物，`package.json` 声明元数据。

## 构建、测试与开发命令
- `cargo build --release`：生成优化二进制到 `target/release/ziro`。
- `cargo run -- <子命令>`：本地调试，例如 `cargo run -- find 8080`。
- `cargo test`：运行所有单元/集成测试。
- `cargo fmt` 与 `cargo clippy -- -D warnings`：格式化并做静态检查。

## 代码风格与测试指引
- Rust 2024 Edition，四空格缩进。模块/函数使用 snake_case，类型/枚举用 PascalCase。
- 错误返回以 `anyhow::Result<T>` 为主；终端输出逻辑集中于 `src/ui/render.rs`。
- 测试：新增功能需补充 `#[cfg(test)] mod tests`。涉及端口和文件删除的测试请使用临时资源并在 `drop` 后清理。
