use crate::process;
use crate::theme::Theme;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub is_symlink: bool,
}

/// 删除错误类型分类
#[derive(Debug, Clone)]
pub enum DeletionError {
    /// 文件被进程占用
    FileLocked(PathBuf),
    /// 权限不足
    PermissionDenied(PathBuf),
    /// 文件不存在
    NotFound(PathBuf),
    /// 目录非空
    DirectoryNotEmpty(PathBuf),
    /// 符号链接指向系统关键路径
    #[allow(dead_code)]
    SystemCriticalLink(PathBuf),
    /// 其他IO错误
    IoError(PathBuf, String),
    /// 其他错误
    #[allow(dead_code)]
    Other(PathBuf, String),
}

impl std::fmt::Display for DeletionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeletionError::FileLocked(path) => {
                write!(f, "文件被进程占用: {}", path.display())
            }
            DeletionError::PermissionDenied(path) => {
                write!(f, "权限不足，无法访问: {}", path.display())
            }
            DeletionError::NotFound(path) => {
                write!(f, "文件或目录不存在: {}", path.display())
            }
            DeletionError::DirectoryNotEmpty(path) => {
                write!(f, "目录非空，无法删除: {}", path.display())
            }
            DeletionError::SystemCriticalLink(path) => {
                write!(f, "符号链接指向系统关键路径，拒绝删除: {}", path.display())
            }
            DeletionError::IoError(path, io_err) => {
                write!(f, "IO错误: {} - {}", path.display(), io_err)
            }
            DeletionError::Other(path, msg) => {
                write!(f, "删除失败: {} - {}", path.display(), msg)
            }
        }
    }
}

impl std::error::Error for DeletionError {}

/// 将io::Error转换为DeletionError
fn classify_deletion_error(path: &Path, error: &io::Error) -> DeletionError {
    match error.kind() {
        io::ErrorKind::PermissionDenied => DeletionError::PermissionDenied(path.to_path_buf()),
        io::ErrorKind::NotFound => DeletionError::NotFound(path.to_path_buf()),
        io::ErrorKind::DirectoryNotEmpty => DeletionError::DirectoryNotEmpty(path.to_path_buf()),
        _ => {
            // Windows特定的错误码检查
            if cfg!(target_os = "windows") {
                let raw_os_error = error.raw_os_error().unwrap_or(0);
                match raw_os_error {
                    // ERROR_SHARING_VIOLATION (32)
                    32 => DeletionError::FileLocked(path.to_path_buf()),
                    // ERROR_ACCESS_DENIED (5)
                    5 => DeletionError::PermissionDenied(path.to_path_buf()),
                    // ERROR_DIR_NOT_EMPTY (145)
                    145 => DeletionError::DirectoryNotEmpty(path.to_path_buf()),
                    _ => DeletionError::IoError(path.to_path_buf(), error.to_string()),
                }
            } else {
                DeletionError::IoError(path.to_path_buf(), error.to_string())
            }
        }
    }
}

/// 提供解决建议
fn get_error_suggestion(error: &DeletionError) -> &'static str {
    match error {
        DeletionError::FileLocked(_) => {
            "建议：关闭占用文件的程序，或使用 --anyway 参数强制终止进程"
        }
        DeletionError::PermissionDenied(_) => "建议：以管理员身份运行，或检查文件权限设置",
        DeletionError::NotFound(_) => "建议：文件可能已被删除或移动",
        DeletionError::DirectoryNotEmpty(_) => {
            "建议：使用 -r/--recursive 参数递归删除，或手动清空目录"
        }
        DeletionError::SystemCriticalLink(_) => "建议：不要删除指向系统关键路径的符号链接",
        DeletionError::IoError(_, _) => "建议：检查文件系统状态，确保磁盘空间充足",
        DeletionError::Other(_, _) => "建议：查看详细错误信息，尝试手动删除",
    }
}

/// 检查路径是否为系统关键目录
pub fn is_system_critical_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();

    if cfg!(target_os = "windows") {
        is_windows_system_critical_path(&path_str)
    } else {
        is_unix_system_critical_path(&path_str)
    }
}

/// Windows系统关键路径检测
fn is_windows_system_critical_path(path_str: &str) -> bool {
    // 只保护真正的系统根目录，允许用户删除其明确指定的任何目录
    let system_critical_paths = get_windows_root_paths();

    system_critical_paths
        .iter()
        .any(|critical| path_str == *critical || path_str.starts_with(&format!("{critical}\\")))
}

/// 获取Windows系统根路径（只保护真正不可删除的系统目录）
fn get_windows_root_paths() -> Vec<String> {
    let mut paths = Vec::new();

    // 系统驱动器根目录（保护整个系统盘根目录）
    if let Ok(system_drive) = std::env::var("SystemDrive") {
        let system_drive_lower = system_drive.to_lowercase();
        paths.push(format!("{}\\", system_drive_lower.trim_end_matches('\\')));
    } else {
        // 回退到默认的C盘根目录
        paths.push("c:\\".to_string());
    }

    paths
}

/// Unix/Linux系统关键路径检测
fn is_unix_system_critical_path(path_str: &str) -> bool {
    // 只保护真正的系统根目录，允许用户删除明确指定的任何其他目录
    let critical_paths = ["/"];

    critical_paths
        .iter()
        .any(|critical| path_str == *critical || path_str.starts_with(&format!("{critical}/")))
}

/// 验证路径是否安全
pub fn validate_paths(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        if is_system_critical_path(path) {
            return Err(anyhow!("不能删除系统关键目录: {}", path.display()));
        }
    }
    Ok(())
}

/// 收集要删除的文件信息
pub fn collect_files_to_remove(paths: &[PathBuf], recursive: bool) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();

    for path in paths {
        if !path.exists() {
            return Err(anyhow!("路径不存在: {}", path.display()));
        }

        if path.is_file() || path.is_symlink() {
            let metadata = path
                .symlink_metadata()
                .with_context(|| format!("无法获取文件元数据: {}", path.display()))?;

            files.push(FileInfo {
                path: path.clone(),
                is_dir: false,
                size: metadata.len(),
                is_symlink: path.is_symlink(),
            });
        } else if path.is_dir() {
            if recursive {
                collect_dir_files(path, &mut files)?;
            } else {
                return Err(anyhow!(
                    "目录删除需要 -r/--recursive 参数: {}",
                    path.display()
                ));
            }
        }
    }

    Ok(files)
}

/// 递归收集目录中的所有文件
fn collect_dir_files(dir: &Path, files: &mut Vec<FileInfo>) -> Result<()> {
    let entries = fs::read_dir(dir).with_context(|| format!("无法读取目录: {}", dir.display()))?;

    // 首先收集所有文件和子目录
    let mut subdirs = Vec::new();

    for entry in entries {
        let entry = entry.with_context(|| format!("无法读取目录项: {}", dir.display()))?;
        let path = entry.path();

        let metadata = entry
            .metadata()
            .with_context(|| format!("无法获取文件元数据: {}", path.display()))?;

        // 检查真正的循环引用（符号链接指向其父目录或祖先目录）
        if path.is_symlink() {
            let target = match fs::read_link(&path) {
                Ok(target) => target,
                Err(_) => {
                    // 无法读取符号链接目标，跳过但记录
                    eprintln!("警告: 无法读取符号链接目标: {}", path.display());
                    continue;
                }
            };

            // 解析相对路径为绝对路径
            let target_path = if target.is_relative() {
                path.parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(&target)
            } else {
                target.clone()
            };

            // 只检查是否真正指向当前处理的目录或其父目录（真正的循环引用）
            if target_path == dir || dir.starts_with(&target_path) {
                eprintln!(
                    "警告: 检测到循环引用，跳过: {} -> {}",
                    path.display(),
                    target_path.display()
                );
                continue;
            }
        }

        if path.is_dir() && !path.is_symlink() {
            // 收集子目录，稍后处理
            subdirs.push(path);
        } else {
            // 添加文件或符号链接
            let is_symlink = path.is_symlink();
            let size = if is_symlink {
                // 对于符号链接，使用符号链接本身的元数据大小
                // 而不是目标文件的大小
                metadata.len()
            } else {
                metadata.len()
            };

            files.push(FileInfo {
                path: path.clone(),
                is_dir: false,
                size,
                is_symlink,
            });
        }
    }

    // 递归处理所有子目录
    for subdir in subdirs {
        collect_dir_files(&subdir, files)?;
    }

    // 最后添加目录本身
    files.push(FileInfo {
        path: dir.to_path_buf(),
        is_dir: true,
        size: 0,
        is_symlink: false,
    });

    Ok(())
}

/// 执行删除操作
pub fn remove_files(
    files: &[FileInfo],
    dry_run: bool,
    verbose: bool,
    anyway: bool,
) -> Vec<(PathBuf, Result<()>)> {
    let theme = Theme::new();
    let mut results = Vec::new();

    // 重新排序文件：先删除所有文件，再删除目录（按深度排序）
    let mut sorted_files = files.to_vec();
    sorted_files.sort_by(|a, b| {
        // 文件优先于目录
        if a.is_dir && !b.is_dir {
            std::cmp::Ordering::Greater
        } else if !a.is_dir && b.is_dir {
            std::cmp::Ordering::Less
        } else {
            // 同类型按路径深度排序，深度小的（父目录）在后面
            let depth_a = a.path.components().count();
            let depth_b = b.path.components().count();
            depth_b.cmp(&depth_a)
        }
    });

    for file in &sorted_files {
        let result = if dry_run {
            Ok(())
        } else {
            remove_single_file(file, verbose, anyway, &theme)
        };

        // 如果是详细模式，显示错误建议
        if let Err(ref e) = result
            && verbose
        {
            eprintln!("{}", theme.icon_error());
            eprintln!("错误: {e}");

            // 尝试提取错误类型并提供建议
            if let Some(deletion_error) = extract_deletion_error(e) {
                eprintln!("建议: {}", get_error_suggestion(&deletion_error));
            }
        }

        results.push((file.path.clone(), result));
    }

    results
}

/// 从错误消息中提取DeletionError
fn extract_deletion_error(error: &anyhow::Error) -> Option<DeletionError> {
    let error_msg = error.to_string();

    if error_msg.contains("文件被进程占用") {
        error
            .chain()
            .find_map(|e| e.downcast_ref::<DeletionError>().cloned())
    } else if error_msg.contains("权限不足") {
        Some(DeletionError::PermissionDenied(PathBuf::from("unknown")))
    } else if error_msg.contains("不存在") {
        Some(DeletionError::NotFound(PathBuf::from("unknown")))
    } else if error_msg.contains("目录非空") {
        Some(DeletionError::DirectoryNotEmpty(PathBuf::from("unknown")))
    } else {
        None
    }
}

/// 删除单个文件，包含重试机制
fn remove_single_file(file: &FileInfo, verbose: bool, anyway: bool, theme: &Theme) -> Result<()> {
    let max_retries = 3;
    let mut last_error = None;

    for attempt in 1..=max_retries {
        let result = attempt_remove_file(file, verbose, anyway, theme, attempt);

        match result {
            Ok(_) => {
                if attempt > 1 && verbose {
                    println!(
                        "{} 第 {} 次尝试成功删除: {}",
                        theme.icon_success(),
                        attempt,
                        file.path.display()
                    );
                }
                return Ok(());
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    if verbose {
                        println!(
                            "{} 第 {} 次尝试删除失败，{}ms后重试: {}",
                            theme.icon_warning(),
                            attempt,
                            1000 * attempt,
                            file.path.display()
                        );
                    }
                    std::thread::sleep(std::time::Duration::from_millis((1000 * attempt) as u64));
                }
            }
        }
    }

    Err(last_error.unwrap())
}

/// 执行单次删除尝试
fn attempt_remove_file(
    file: &FileInfo,
    verbose: bool,
    anyway: bool,
    theme: &Theme,
    attempt: u32,
) -> Result<()> {
    // 检查文件是否被占用
    if process::is_file_locked(&file.path) {
        if anyway {
            // 尝试终止占用进程
            match process::find_processes_by_file(&file.path) {
                Ok(pids) if !pids.is_empty() => {
                    if verbose && attempt == 1 {
                        println!(
                            "{} 文件被占用，终止进程并删除: {} (占用进程: {})",
                            theme.icon_fire(),
                            file.path.display(),
                            pids.iter()
                                .map(|p| p.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }

                    // 强制终止占用进程
                    let kill_results = process::kill_processes_force(&pids);
                    let killed_count = kill_results.iter().filter(|(_, r)| r.is_ok()).count();

                    if killed_count > 0 && verbose {
                        println!(
                            "{} 成功终止 {}/{} 个占用进程",
                            theme.icon_success(),
                            killed_count,
                            pids.len()
                        );
                    }

                    // 等待进程完全退出
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
                Ok(_) => {
                    // 没有找到占用进程，但检测到被占用
                    if attempt == 1 {
                        return Err(anyhow!(
                            "文件被占用但无法识别占用进程: {}",
                            file.path.display()
                        ));
                    }
                }
                Err(e) => {
                    if attempt == 1 {
                        return Err(anyhow!("查找占用进程失败: {} - {}", e, file.path.display()));
                    }
                }
            }
        } else {
            return Err(anyhow!(
                "文件被进程占用，使用 --anyway 参数强制删除: {}",
                file.path.display()
            ));
        }
    }

    // 执行实际删除
    if file.is_dir {
        remove_directory_with_fallback(&file.path)
    } else {
        remove_file_with_fallback(&file.path)
    }
}

/// 带降级策略的文件删除
fn remove_file_with_fallback(path: &Path) -> Result<()> {
    // 确保是符号链接时只删除链接本身
    if path.is_symlink() {
        return remove_symlink_safely(path);
    }

    // 方法1：正常删除
    match fs::remove_file(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            let deletion_error = classify_deletion_error(path, &e);

            // 对于某些错误类型，尝试降级策略
            match deletion_error {
                DeletionError::PermissionDenied(_) => {
                    // 尝试修改文件属性后删除
                    if try_remove_readonly_attribute(path) && fs::remove_file(path).is_ok() {
                        Ok(())
                    } else {
                        Err(anyhow!("{deletion_error}"))
                    }
                }
                DeletionError::FileLocked(_) => {
                    // 文件锁定错误将在上层处理
                    Err(anyhow!("{deletion_error}"))
                }
                _ => {
                    // 其他错误类型，继续后续处理
                    Err(anyhow!("{deletion_error}"))
                }
            }
        }
    }
}

/// 尝试去除文件的只读属性
fn try_remove_readonly_attribute(path: &Path) -> bool {
    if cfg!(target_os = "windows") {
        match std::process::Command::new("attrib")
            .args(["-R", &path.to_string_lossy()])
            .status()
        {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    } else {
        // Windows系统：没有简单的权限修改方法，返回false
        false
    }
}

/// 安全地删除符号链接（只删除链接本身，不影响目标）
fn remove_symlink_safely(path: &Path) -> Result<()> {
    // 确认这确实是一个符号链接
    if !path.is_symlink() {
        return Err(anyhow!("路径不是符号链接: {}", path.display()));
    }

    // 在删除前检查链接目标是否存在，以及是否为敏感路径
    if let Ok(target) = fs::read_link(path) {
        let target_path = if target.is_absolute() {
            target.clone()
        } else {
            // 相对路径，需要基于符号链接所在目录解析
            path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&target)
        };

        // 检查是否链接到系统关键目录
        if crate::file::is_system_critical_path(&target_path) {
            eprintln!(
                "警告: 符号链接指向系统关键路径，跳过删除: {} -> {}",
                path.display(),
                target_path.display()
            );
            return Err(anyhow!("符号链接指向系统关键路径: {}", path.display()));
        }

        // 记录即将删除的符号链接信息
        eprintln!(
            "删除符号链接: {} -> {}",
            path.display(),
            target_path.display()
        );
    }

    // 删除符号链接本身
    fs::remove_file(path).with_context(|| format!("无法删除符号链接: {}", path.display()))
}

/// 带降级策略的目录删除
fn remove_directory_with_fallback(path: &Path) -> Result<()> {
    // 处理符号链接目录
    if path.is_symlink() {
        return remove_symlink_directory_safely(path);
    }

    // 确保目录为空
    match fs::read_dir(path) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(anyhow!("目录非空，无法删除: {}", path.display()));
            }
        }
        Err(_) => {
            // 无法读取目录，可能已经被删除
            return Ok(());
        }
    }

    // 方法1：正常删除
    if fs::remove_dir(path).is_ok() {
        return Ok(());
    }

    // 方法2：在Windows上尝试去除只读属性
    if cfg!(target_os = "windows")
        && std::process::Command::new("attrib")
            .args(["-R", &path.to_string_lossy()])
            .status()
            .is_ok()
        && fs::remove_dir(path).is_ok()
    {
        return Ok(());
    }

    // 方法3：最后尝试
    fs::remove_dir(path).with_context(|| format!("无法删除目录: {}", path.display()))
}

/// 安全地删除符号链接目录
fn remove_symlink_directory_safely(path: &Path) -> Result<()> {
    // 确认这确实是一个符号链接目录
    if !path.is_symlink() {
        return Err(anyhow!("路径不是符号链接目录: {}", path.display()));
    }

    // 在删除前检查链接目标
    if let Ok(target) = fs::read_link(path) {
        let target_path = if target.is_absolute() {
            target.clone()
        } else {
            // 相对路径，需要基于符号链接所在目录解析
            path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&target)
        };

        // 检查是否链接到系统关键目录
        if crate::file::is_system_critical_path(&target_path) {
            eprintln!(
                "警告: 符号链接目录指向系统关键路径，跳过删除: {} -> {}",
                path.display(),
                target_path.display()
            );
            return Err(anyhow!("符号链接目录指向系统关键路径: {}", path.display()));
        }

        // 记录即将删除的符号链接目录信息
        eprintln!(
            "删除符号链接目录: {} -> {}",
            path.display(),
            target_path.display()
        );
    }

    // 删除符号链接目录本身（使用remove_file而不是remove_dir）
    fs::remove_file(path).with_context(|| format!("无法删除符号链接目录: {}", path.display()))
}

/// 格式化文件大小
pub fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}
