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

/// 检查路径是否为系统关键目录
pub fn is_system_critical_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();

    let critical_paths = if cfg!(target_os = "windows") {
        vec![
            "c:\\",
            "c:\\windows",
            "c:\\windows\\system32",
            "c:\\program files",
            "c:\\program files (x86)",
            "c:\\users",
            "c:\\documents and settings",
        ]
    } else {
        vec![
            "/",
            "/bin",
            "/sbin",
            "/usr/bin",
            "/usr/sbin",
            "/etc",
            "/var",
            "/sys",
            "/proc",
            "/boot",
            "/lib",
            "/lib64",
        ]
    };

    critical_paths
        .iter()
        .any(|critical| path_str.starts_with(critical) || path_str.as_str() == *critical)
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

    for entry in entries {
        let entry = entry.with_context(|| format!("无法读取目录项: {}", dir.display()))?;
        let path = entry.path();

        let metadata = entry
            .metadata()
            .with_context(|| format!("无法获取文件元数据: {}", path.display()))?;

        if path.is_dir() {
            // 先递归处理子目录
            collect_dir_files(&path, files)?;
        } else {
            // 添加文件
            files.push(FileInfo {
                path: path.clone(),
                is_dir: false,
                size: metadata.len(),
                is_symlink: path.is_symlink(),
            });
        }
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
pub fn remove_files(files: &[FileInfo], dry_run: bool) -> Vec<(PathBuf, Result<()>)> {
    let mut results = Vec::new();

    for file in files {
        let result = if dry_run {
            Ok(())
        } else if file.is_dir {
            fs::remove_dir(&file.path)
                .with_context(|| format!("无法删除目录: {}", file.path.display()))
        } else {
            fs::remove_file(&file.path)
                .with_context(|| format!("无法删除文件: {}", file.path.display()))
        };

        results.push((file.path.clone(), result));
    }

    results
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
