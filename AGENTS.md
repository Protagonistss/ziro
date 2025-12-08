# Repository Guidelines

## 项目结构与模块组织
- Rust 核心：入口在 `src/bin/ziro.rs`；`src/cli/`（`args.rs` 参数定义，`handlers.rs` 命令分发）；`src/core/`（`port/` 端口扫描，`process/` 进程终止，`fs_ops/` 文件删除与安全校验，`top/` 监控）；`src/platform/`（`term.rs` 终端/环境配置，`encoding.rs` Windows UTF-8 初始化）；`src/ui/`（`render.rs` 输出、交互，`theme.rs` 颜色，`icons.rs` 图标）。
- Node 分发层：`bin/ziro.js` 作为 npm 启动代理，`scripts/install.js`+`detect-platform.js` 下载发布产物，`package.json` 声明元数据。
- 发行/工具：`target/` 存放构建产物（已忽略），`.github/` 存放 CI/发布配置。

## 构建、测试与开发命令
- `cargo build --release`：生成优化二进制到 `target/release/ziro`。
- `cargo run -- <子命令>`：本地调试，例如 `cargo run -- find 8080`、`cargo run -- top --once --limit 5`。
- `cargo test`：运行所有单元/集成测试。
- `cargo fmt` 与 `cargo clippy -- -D warnings`：格式化并做静态检查。
- npm 打包验证：`npm install -g .` 或 `npm pack`（触发 `scripts/install.js` 下载对应平台二进制）。

## 代码风格与命名约定
- Rust 2024 Edition，四空格缩进；模块/函数使用 snake_case，类型/枚举用 PascalCase，常量 SCREAMING_SNAKE_CASE。
- 错误返回以 `anyhow::Result<T>` 为主；终端输出逻辑集中于 `src/ui/render.rs`，复用 `theme.rs`、`icons.rs` 保持一致性。
- 环境变量控制 UI：`ZIRO_PLAIN`（纯 ASCII）、`ZIRO_ASCII_ICONS`、`ZIRO_UNICODE_ICONS`、`ZIRO_NO_COLOR`、`ZIRO_NARROW` 等，新增行为时保持兼容并更新文档。

## 测试指引
- 现有测试较少（仅 `src/icons.rs`），新增功能需补充 `#[cfg(test)] mod tests`；CLI 行为建议放在 `tests/` 做集成测试，覆盖端口查找、强制删除、Top 模式等关键路径。
- 涉及端口和文件删除的测试请使用临时资源并在 `drop` 后清理，避免污染宿主环境；提交前至少跑 `cargo test` 与 `cargo clippy`。

## 提交与 PR 规范
- Git 历史采用接近 Conventional Commits：`feat(core): ...`、`refactor(term): ...`、`chore(version): ...`。保持相同前缀+作用域+动宾短语。
- PR 需包含：变更摘要、测试命令与结果、受影响的命令示例或兼容性说明、终端 UI 截图/录屏（如有）、关联 Issue；尽量保持中英文用户提示一致。

## 安全与配置
- 删除/终止流程在 `src/core/fs_ops/`、`src/core/process/` 已做确认，默认交互式；只有在自动化场景才加 `--force`/`--anyway`。
- Windows UTF-8 初始化在 `src/platform/encoding.rs`，入口调用；Node 安装脚本依赖 GitHub Releases，离线部署需提前下载对应压缩包放入 `bin/`。
