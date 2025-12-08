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

fn remove_entry(file: &FileInfo) -> Result<()> {
    if file.is_symlink {
        fs::remove_file(&file.path)?;
        return Ok(());
    }

    if file.is_dir {
        fs::remove_dir(&file.path)?;
    } else {
        fs::remove_file(&file.path)?;
    }
    Ok(())
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
