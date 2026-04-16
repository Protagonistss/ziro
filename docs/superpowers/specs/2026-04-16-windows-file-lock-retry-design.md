# Windows 文件删除占用问题改进设计

## 问题

`ziro remove` 在 Windows 上删除文件/目录时，遇到进程占用会失败，存在两个根本问题：

1. **检测不准确** — 当前 `find_processes_by_file` 依赖 PowerShell/wmic/handle.exe，只能匹配进程的可执行文件路径，无法检测通过 `CreateFile` 打开文件句柄的进程
2. **杀进程后仍删不掉** — 终止占用进程后文件句柄释放需要时间，当前代码没有重试机制，一次失败就报错

## 方案

终止进程 + 指数退避重试，对用户透明。

## 设计细节

### 1. 使用 RestartManager API 替代 PowerShell 检测

Windows RestartManager API（`RmStartSession` / `RmRegisterResources` / `RmGetList`）是系统级 API，能精确查找持有文件句柄的进程。这是 Windows 资源管理器提示"文件被占用"时使用的同一套 API。

**Cargo.toml 改动：**
- `windows-sys` 新增 feature：`Win32_System_RestartManager`

**lock.rs 新增函数：**
```rust
#[cfg(target_os = "windows")]
fn find_processes_with_restart_manager(path: &Path) -> Result<Vec<u32>>
```

**检测优先级链：** RestartManager（主） → PowerShell（兼容兜底）

**清理范围：** 移除 `find_processes_with_wmic` 函数（wmic 在新版 Windows 已废弃）。

### 2. 删除重试机制

在 `remove_entry` 和 `try_windows_bulk_remove` 中包裹指数退避重试逻辑。

**参数：**
- 最大重试次数：5
- 初始等待：100ms
- 指数因子：2（等待序列 100ms, 200ms, 400ms, 800ms, 1000ms）
- 最大单次等待：1000ms

**重试条件（仅以下错误触发重试）：**
- `ErrorKind::PermissionDenied`
- Windows 错误码 32（ERROR_SHARING_VIOLATION）
- Windows 错误码 33（ERROR_LOCK_VIOLATION）
- Windows 错误码 5（ERROR_ACCESS_DENIED）

**重试时每次失败自动重新检测占用进程并尝试终止。**

**新增函数：**
```rust
#[cfg(target_os = "windows")]
fn remove_with_retry(file: &FileInfo) -> Result<()>

#[cfg(target_os = "windows")]
fn is_retryable_error(e: &std::io::Error) -> bool
```

### 3. 交互流程

改进后流程：
1. 检查占用（使用 RestartManager，更准确）
2. 有占用 → 提示用户 / `--anyway` 自动终止
3. 终止进程后删除，带指数退避重试
4. 重试过程中每次失败自动重新检测并尝试终止新出现的占用进程
5. 所有重试用尽仍失败 → 报错，显示哪个进程仍在占用

### 4. 非侵入性约束

- 所有改动仅影响 `#[cfg(target_os = "windows")]` 代码块
- Unix 端行为不变
- `--anyway` 参数行为不变，内部实现更可靠
- 不新增 CLI 参数，不改变用户接口

## 改动文件清单

| 文件 | 改动内容 |
|------|----------|
| `Cargo.toml` | `windows-sys` 新增 `Win32_System_RestartManager` feature |
| `src/core/process/lock.rs` | 新增 `find_processes_with_restart_manager`，移除 `find_processes_with_wmic` |
| `src/core/fs_ops/mod.rs` | 新增 `remove_with_retry`、`is_retryable_error`，改造 `remove_entry` 和 `try_windows_bulk_remove` |

## 不做的事

- 不做重启后删除（MoveFileEx + MOVEFILE_DELAY_UNTIL_REBOOT）
- 不新增 CLI 参数
- 不改 Unix 端逻辑
