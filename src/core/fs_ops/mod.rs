use crate::ui::Theme;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub is_symlink: bool,
}

/// 验证路径是否存在
pub fn validate_paths(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        if !path.exists() {
            return Err(anyhow!("路径不存在: {}", path.display()));
        }
    }
    Ok(())
}

/// 收集待删除的文件/目录信息
pub fn collect_files_to_remove(paths: &[PathBuf], recursive: bool) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();

    for path in paths {
        let metadata = path
            .symlink_metadata()
            .with_context(|| format!("无法获取文件元数据: {}", path.display()))?;
        let is_symlink = metadata.file_type().is_symlink();
        let is_dir = metadata.is_dir() && !is_symlink;

        if is_dir {
            if recursive {
                collect_dir_files(path, &mut files)?;
                files.push(FileInfo {
                    path: path.clone(),
                    is_dir: true,
                    size: 0,
                    is_symlink: false,
                });
            } else {
                // 非递归模式仅允许空目录
                if path.read_dir()?.next().is_some() {
                    return Err(anyhow!(
                        "目录删除需要 -r/--recursive 参数: {}",
                        path.display()
                    ));
                }
                files.push(FileInfo {
                    path: path.clone(),
                    is_dir: true,
                    size: 0,
                    is_symlink: false,
                });
            }
        } else {
            files.push(FileInfo {
                path: path.clone(),
                is_dir: false,
                size: metadata.len(),
                is_symlink,
            });
        }
    }

    Ok(files)
}

/// 递归收集目录内容（不跟随符号链接）
fn collect_dir_files(dir: &Path, files: &mut Vec<FileInfo>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("无法读取目录: {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("无法读取目录项: {}", dir.display()))?;
        let path = entry.path();
        let metadata = path
            .symlink_metadata()
            .with_context(|| format!("无法获取文件元数据: {}", path.display()))?;
        let is_symlink = metadata.file_type().is_symlink();
        let is_dir = metadata.is_dir() && !is_symlink;

        if is_dir {
            collect_dir_files(&path, files)?;
            files.push(FileInfo {
                path,
                is_dir: true,
                size: 0,
                is_symlink: false,
            });
        } else {
            files.push(FileInfo {
                path,
                is_dir: false,
                size: metadata.len(),
                is_symlink,
            });
        }
    }

    Ok(())
}

/// 执行删除
pub fn remove_files(
    files: &[FileInfo],
    dry_run: bool,
    verbose: bool,
    _anyway: bool,
) -> Vec<(PathBuf, Result<()>)> {
    let theme = Theme::new();
    let mut results = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // Windows 特殊处理：查找用户直接指定的根目录
        // 在 collect_files_to_remove 中，根目录是最后添加的
        if let Some(root_dir) = files.iter().find(|f| {
            f.is_dir
                && !files
                    .iter()
                    .any(|other| other.path != f.path && f.path.starts_with(&other.path))
        }) {
            if !dry_run {
                // 尝试直接使用 remove_dir_all 删除整个目录树
                match remove_dir_all_with_symlinks(&root_dir.path) {
                    Ok(_) => {
                        if verbose {
                            println!(
                                "{} {}",
                                theme.icon_success(),
                                theme.muted(format!("删除 {}", root_dir.path.display()))
                            );
                        }
                        results.push((root_dir.path.clone(), Ok(())));
                        return results;
                    }
                    Err(e) => {
                        if verbose {
                            println!(
                                "{} {}",
                                theme.icon_warning(),
                                theme.warning(format!("批量删除失败，尝试逐个删除: {}", e))
                            );
                        }
                        // 如果批量删除失败，继续逐个删除
                        // 跳出 Windows 特殊处理，使用常规逐个删除
                    }
                }
            } else {
                // Dry run 模式，直接返回
                results.push((root_dir.path.clone(), Ok(())));
                return results;
            }
        }
    }

    // 确保先删文件后删目录（深度优先）
    let mut sorted = files.to_vec();
    sorted.sort_by(|a, b| {
        if a.is_dir && !b.is_dir {
            std::cmp::Ordering::Greater
        } else if !a.is_dir && b.is_dir {
            std::cmp::Ordering::Less
        } else {
            let depth_a = a.path.components().count();
            let depth_b = b.path.components().count();
            depth_b.cmp(&depth_a)
        }
    });

    for file in sorted {
        let result = if dry_run {
            Ok(())
        } else {
            remove_entry(&file).with_context(|| format!("删除失败: {}", file.path.display()))
        };

        if verbose {
            match &result {
                Ok(_) => println!(
                    "{} {}",
                    theme.icon_success(),
                    theme.muted(format!("删除 {}", file.path.display()))
                ),
                Err(e) => println!(
                    "{} {}",
                    theme.icon_error(),
                    theme.error(format!("删除失败 {} - {}", file.path.display(), e))
                ),
            }
        }

        results.push((file.path, result));
    }

    results
}

/// Windows 上删除包含符号链接的目录
#[cfg(target_os = "windows")]
fn remove_dir_all_with_symlinks(path: &Path) -> Result<()> {
    // 先尝试移除所有只读属性
    if let Err(_) = remove_readonly_recursively(path) {
        // 忽略错误，继续尝试删除
    }

    // 使用 remove_dir_all，这在 Windows 上可以处理符号链接
    fs::remove_dir_all(path).with_context(|| format!("无法删除目录: {}", path.display()))
}

/// 递归移除目录及其内容的只读属性
#[cfg(target_os = "windows")]
fn remove_readonly_recursively(path: &Path) -> Result<()> {
    let metadata = path
        .symlink_metadata()
        .with_context(|| format!("无法获取路径元数据: {}", path.display()))?;

    // 只处理文件和目录，不处理符号链接
    if !metadata.file_type().is_symlink() {
        #[allow(clippy::permissions_set_readonly_false)]
        {
            let mut perms = metadata.permissions();
            perms.set_readonly(false);
            fs::set_permissions(path, perms)
                .with_context(|| format!("无法设置权限: {}", path.display()))?;
        }

        if metadata.is_dir() {
            for entry in
                fs::read_dir(path).with_context(|| format!("无法读取目录: {}", path.display()))?
            {
                let entry = entry.with_context(|| format!("无法读取目录项: {}", path.display()))?;
                remove_readonly_recursively(&entry.path())?;
            }
        }
    }
    Ok(())
}

fn remove_entry(file: &FileInfo) -> Result<()> {
    // 在 Windows 上，处理符号链接需要特殊处理
    #[cfg(target_os = "windows")]
    {
        if file.is_symlink {
            // 对于符号链接，始终使用 remove_file
            // 这会删除链接本身，而不是目标
            match fs::remove_file(&file.path) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    // 如果失败，尝试使用 Windows 特定的方法
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        // 尝试获取文件属性并移除只读属性
                        if let Ok(metadata) = file.path.metadata() {
                            #[allow(clippy::permissions_set_readonly_false)]
                            {
                                let mut attrs = metadata.permissions();
                                attrs.set_readonly(false);
                                if let Err(_) = fs::set_permissions(&file.path, attrs) {
                                    // 如果无法修改权限，继续尝试删除
                                }
                            }
                        }
                        // 再次尝试删除
                        return fs::remove_file(&file.path)
                            .with_context(|| format!("无法删除符号链接: {}", file.path.display()));
                    }
                    return Err(e.into());
                }
            }
        }
    }

    // 非符号链接的常规处理
    let result = if file.is_symlink {
        fs::remove_file(&file.path)
    } else if file.is_dir {
        // 对于目录，先尝试 remove_dir（空目录）
        match fs::remove_dir(&file.path) {
            Ok(_) => Ok(()),
            Err(e) => {
                // 如果是权限错误，尝试修改权限后再删除
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    // 递归修改目录及其内容的权限
                    #[cfg(target_os = "windows")]
                    if let Err(_) = remove_readonly_recursively(&file.path) {
                        // 如果无法修改权限，继续尝试删除
                    }
                    // 再次尝试删除
                    fs::remove_dir_all(&file.path)
                } else {
                    Err(e)
                }
            }
        }
    } else {
        fs::remove_file(&file.path)
    };

    result.with_context(|| format!("删除失败: {}", file.path.display()))
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
