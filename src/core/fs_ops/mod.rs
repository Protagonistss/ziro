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

    // Windows 特殊处理：尝试批量删除
    #[cfg(target_os = "windows")]
    if let Some(results) = try_windows_bulk_remove(files, dry_run, verbose, &theme) {
        return results;
    }

    // 通用逐个删除逻辑
    remove_files_individually(files, dry_run, verbose, &theme)
}

/// Windows 特殊处理：尝试批量删除根目录
#[cfg(target_os = "windows")]
fn try_windows_bulk_remove(
    files: &[FileInfo],
    dry_run: bool,
    verbose: bool,
    theme: &Theme,
) -> Option<Vec<(PathBuf, Result<()>)>> {
    // 查找用户直接指定的根目录
    let root_dir = files.iter().find(|f| {
        f.is_dir
            && !files
                .iter()
                .any(|other| other.path != f.path && f.path.starts_with(&other.path))
    })?;

    if dry_run {
        return Some(vec![(root_dir.path.clone(), Ok(()))]);
    }

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
            Some(vec![(root_dir.path.clone(), Ok(()))])
        }
        Err(e) => {
            if verbose {
                println!(
                    "{} {}",
                    theme.icon_warning(),
                    theme.warning(format!("批量删除失败，尝试逐个删除: {}", e))
                );
            }
            // 批量删除失败，返回 None 让调用者使用逐个删除
            None
        }
    }
}

/// 逐个删除文件（通用逻辑）
fn remove_files_individually(
    files: &[FileInfo],
    dry_run: bool,
    verbose: bool,
    theme: &Theme,
) -> Vec<(PathBuf, Result<()>)> {
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

/// Windows 上删除包含符号链接的目录
#[cfg(target_os = "windows")]
fn remove_dir_all_with_symlinks(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;

    // 转换为 Windows 长路径格式 (\\?\ 前缀)
    // 这允许绕过 MAX_PATH (260 字符) 限制
    let long_path = to_long_path(path)?;

    // 使用 Windows API 删除目录
    unsafe {
        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::Storage::FileSystem::{
            DeleteFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY,
            FILE_ATTRIBUTE_REPARSE_POINT, GetFileAttributesW, INVALID_FILE_ATTRIBUTES,
            RemoveDirectoryW, SetFileAttributesW,
        };

        let path_wide: Vec<u16> = long_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // 获取文件属性
        let attrs = GetFileAttributesW(path_wide.as_ptr());
        if attrs == INVALID_FILE_ATTRIBUTES {
            return Err(anyhow!("无法获取文件属性: {}", path.display()));
        }

        // 检查是否为符号链接（重解析点）
        let is_reparse_point = (attrs & FILE_ATTRIBUTE_REPARSE_POINT) != 0;

        if is_reparse_point {
            // 对于符号链接，使用 DeleteFileW
            if DeleteFileW(path_wide.as_ptr()) == 0 {
                let err = GetLastError();
                return Err(anyhow!(
                    "无法删除符号链接: {}, 错误代码: {}",
                    path.display(),
                    err
                ));
            }
        } else if (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0 {
            // 对于目录，递归删除内容
            match remove_directory_recursive(&long_path) {
                Ok(_) => {}
                Err(e) => {
                    let err = GetLastError();
                    return Err(anyhow!(
                        "递归删除目录内容失败: {}, 原始错误: {}, Windows错误代码: {}",
                        path.display(),
                        e,
                        err
                    ));
                }
            }

            // 移除根目录的只读属性
            SetFileAttributesW(path_wide.as_ptr(), attrs & !FILE_ATTRIBUTE_READONLY);

            // 删除空目录
            if RemoveDirectoryW(path_wide.as_ptr()) == 0 {
                let err = GetLastError();
                return Err(anyhow!(
                    "无法删除目录: {}, Windows错误代码: {}",
                    path.display(),
                    err
                ));
            }
        } else {
            // 移除文件的只读属性
            SetFileAttributesW(path_wide.as_ptr(), attrs & !FILE_ATTRIBUTE_READONLY);

            // 对于文件，使用 DeleteFileW
            if DeleteFileW(path_wide.as_ptr()) == 0 {
                let err = GetLastError();
                return Err(anyhow!(
                    "无法删除文件: {}, Windows错误代码: {}",
                    path.display(),
                    err
                ));
            }
        }
    }

    Ok(())
}

/// 递归删除目录内容（使用长路径）
#[cfg(target_os = "windows")]
unsafe fn remove_directory_recursive(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        DeleteFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY,
        FILE_ATTRIBUTE_REPARSE_POINT, FindClose, FindFirstFileW, FindNextFileW, RemoveDirectoryW,
        SetFileAttributesW, WIN32_FIND_DATAW,
    };

    // 构建搜索模式：路径\*
    let mut search_pattern = path.to_path_buf();
    search_pattern.push("*");
    let search_wide: Vec<u16> = search_pattern
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut find_data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    let find_handle = unsafe { FindFirstFileW(search_wide.as_ptr(), &mut find_data) };

    if find_handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        // 目录为空或出错，返回成功
        return Ok(());
    }

    loop {
        // 跳过 . 和 ..
        let name = find_data.cFileName[..]
            .iter()
            .take_while(|&&c| c != 0)
            .copied()
            .collect::<Vec<_>>();
        let name_str = String::from_utf16_lossy(&name);

        if name_str != "." && name_str != ".." {
            let mut item_path = path.to_path_buf();
            item_path.push(&name_str);

            let item_wide: Vec<u16> = item_path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let is_dir = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
            let is_reparse_point = (find_data.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0;

            if is_dir && !is_reparse_point {
                // 递归删除子目录
                unsafe {
                    remove_directory_recursive(&item_path)?;
                }
                // 移除只读属性
                unsafe {
                    SetFileAttributesW(
                        item_wide.as_ptr(),
                        find_data.dwFileAttributes & !FILE_ATTRIBUTE_READONLY,
                    );
                }
                // 删除空目录
                if unsafe { RemoveDirectoryW(item_wide.as_ptr()) } == 0 {
                    unsafe {
                        FindClose(find_handle);
                    }
                    return Err(anyhow!("无法删除目录: {}", item_path.display()));
                }
            } else {
                // 移除只读属性
                unsafe {
                    SetFileAttributesW(
                        item_wide.as_ptr(),
                        find_data.dwFileAttributes & !FILE_ATTRIBUTE_READONLY,
                    );
                }
                // 删除文件或符号链接
                if unsafe { DeleteFileW(item_wide.as_ptr()) } == 0 {
                    unsafe {
                        FindClose(find_handle);
                    }
                    return Err(anyhow!("无法删除文件: {}", item_path.display()));
                }
            }
        }

        if unsafe { FindNextFileW(find_handle, &mut find_data) } == 0 {
            break;
        }
    }

    unsafe {
        FindClose(find_handle);
    }
    Ok(())
}

/// 将路径转换为 Windows 长路径格式
#[cfg(target_os = "windows")]
fn to_long_path(path: &Path) -> Result<PathBuf> {
    // 尝试获取绝对路径，如果失败则使用原始路径
    let absolute = match fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => {
            // 如果 canonicalize 失败（可能是路径太长），使用绝对路径
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()
                    .ok()
                    .map(|cwd| cwd.join(path))
                    .unwrap_or_else(|| path.to_path_buf())
            }
        }
    };

    // 检查是否已经是 UNC 路径
    let path_str = absolute.to_string_lossy().to_string();
    let has_prefix = path_str.starts_with(r"\\?\") || path_str.starts_with(r"\\?\UNC\");

    if has_prefix {
        return Ok(absolute);
    }

    // 添加 \\?\ 前缀
    let long_path = if let Some(stripped) = path_str.strip_prefix(r"\\") {
        // UNC 路径：\\?\UNC\server\share
        PathBuf::from(format!(r"\\?\UNC\{}", stripped))
    } else {
        // 普通路径：\\?\C:\path
        PathBuf::from(format!(r"\\?{}", path_str))
    };

    Ok(long_path)
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
