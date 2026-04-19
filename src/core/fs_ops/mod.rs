use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

/// Windows deletion retry parameters
#[cfg(target_os = "windows")]
const RETRY_MAX_ATTEMPTS: u32 = 5;
#[cfg(target_os = "windows")]
const RETRY_INITIAL_WAIT_MS: u64 = 100;
#[cfg(target_os = "windows")]
const RETRY_MAX_WAIT_MS: u64 = 1000;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub is_symlink: bool,
}

/// Validate that paths exist
pub fn validate_paths(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        if !path.exists() {
            return Err(anyhow!("Path does not exist: {}", path.display()));
        }
    }
    Ok(())
}

/// Collect file/directory info for removal
pub fn collect_files_to_remove(paths: &[PathBuf], recursive: bool) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();

    for path in paths {
        let metadata = path
            .symlink_metadata()
            .with_context(|| format!("Failed to get file metadata: {}", path.display()))?;
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
                // Non-recursive mode: only allow empty directories
                if path.read_dir()?.next().is_some() {
                    return Err(anyhow!(
                        "Directory requires -r/--recursive flag: {}",
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

/// Recursively collect directory contents (does not follow symlinks)
fn collect_dir_files(dir: &Path, files: &mut Vec<FileInfo>) -> Result<()> {
    for entry in
        fs::read_dir(dir).with_context(|| format!("Failed to read directory: {}", dir.display()))?
    {
        let entry =
            entry.with_context(|| format!("Failed to read directory entry: {}", dir.display()))?;
        let path = entry.path();
        let metadata = path
            .symlink_metadata()
            .with_context(|| format!("Failed to get file metadata: {}", path.display()))?;
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

/// Execute deletion
pub fn remove_files(
    files: &[FileInfo],
    dry_run: bool,
    _verbose: bool,
    anyway: bool,
) -> Vec<(PathBuf, Result<()>)> {
    // Windows special handling: try bulk deletion
    #[cfg(target_os = "windows")]
    if let Some(results) = try_windows_bulk_remove(files, dry_run, anyway) {
        return results;
    }

    // Generic individual deletion logic
    remove_files_individually(files, dry_run, anyway)
}

/// Windows special handling: try bulk deletion of root directory
#[cfg(target_os = "windows")]
fn try_windows_bulk_remove(
    files: &[FileInfo],
    dry_run: bool,
    anyway: bool,
) -> Option<Vec<(PathBuf, Result<()>)>> {
    let root_dir = files.iter().find(|f| {
        f.is_dir
            && !files
                .iter()
                .any(|other| other.path != f.path && f.path.starts_with(&other.path))
    })?;

    if dry_run {
        return Some(vec![(root_dir.path.clone(), Ok(()))]);
    }

    // Try to use remove_dir_all to delete the entire directory tree, with retries
    #[cfg(target_os = "windows")]
    use crate::core::process::{find_processes_by_file, kill_process_force};

    let mut wait_ms = RETRY_INITIAL_WAIT_MS;
    let mut last_err = None;
    let mut success = false;

    for attempt in 0..=RETRY_MAX_ATTEMPTS {
        match remove_dir_all_with_symlinks(&root_dir.path) {
            Ok(_) => {
                success = true;
                break;
            }
            Err(e) => {
                last_err = Some(e);

                let io_err = last_err
                    .as_ref()
                    .and_then(|e| e.downcast_ref::<std::io::Error>());
                let should_retry = io_err.is_some_and(is_retryable_error);

                if !should_retry || attempt == RETRY_MAX_ATTEMPTS {
                    break;
                }

                if anyway {
                    if let Ok(pids) = find_processes_by_file(&root_dir.path) {
                        for pid in pids {
                            let _ = kill_process_force(pid);
                        }
                    }
                }

                eprintln!("  Retrying ({}/{})...", attempt + 1, RETRY_MAX_ATTEMPTS);

                std::thread::sleep(std::time::Duration::from_millis(wait_ms));
                wait_ms = (wait_ms * 2).min(RETRY_MAX_WAIT_MS);
            }
        }
    }

    if success {
        Some(vec![(root_dir.path.clone(), Ok(()))])
    } else {
        eprintln!(
            "  Bulk delete failed, trying individual deletion: {}",
            last_err.unwrap_or_else(|| anyhow::anyhow!("Unknown error"))
        );
        None
    }
}

/// Delete files individually (generic logic)
fn remove_files_individually(
    files: &[FileInfo],
    dry_run: bool,
    anyway: bool,
) -> Vec<(PathBuf, Result<()>)> {
    let mut results = Vec::new();

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
            remove_with_retry(&file, anyway)
        };

        results.push((file.path, result));
    }

    results
}

/// Delete directories containing symlinks on Windows
#[cfg(target_os = "windows")]
fn remove_dir_all_with_symlinks(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;

    // Convert to Windows long path format (\\?\ prefix)
    // This bypasses the MAX_PATH (260 character) limit
    let long_path = to_long_path(path)?;

    // Use Windows API to delete directory
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

        // Get file attributes
        let attrs = GetFileAttributesW(path_wide.as_ptr());
        if attrs == INVALID_FILE_ATTRIBUTES {
            return Err(anyhow!("Failed to get file attributes: {}", path.display()));
        }

        // Check if it is a symlink (reparse point)
        let is_reparse_point = (attrs & FILE_ATTRIBUTE_REPARSE_POINT) != 0;

        if is_reparse_point {
            // For symlinks, use DeleteFileW
            if DeleteFileW(path_wide.as_ptr()) == 0 {
                let err = GetLastError();
                return Err(std::io::Error::from_raw_os_error(err as i32))
                    .with_context(|| format!("Failed to delete symlink: {}", path.display()));
            }
        } else if (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0 {
            // For directories, recursively delete contents
            remove_directory_recursive(&long_path).with_context(|| {
                format!(
                    "Failed to recursively delete directory contents: {}",
                    path.display()
                )
            })?;

            // Remove read-only attribute from root directory
            SetFileAttributesW(path_wide.as_ptr(), attrs & !FILE_ATTRIBUTE_READONLY);

            // Delete empty directory
            if RemoveDirectoryW(path_wide.as_ptr()) == 0 {
                let err = GetLastError();
                return Err(std::io::Error::from_raw_os_error(err as i32))
                    .with_context(|| format!("Failed to delete directory: {}", path.display()));
            }
        } else {
            // Remove read-only attribute from file
            SetFileAttributesW(path_wide.as_ptr(), attrs & !FILE_ATTRIBUTE_READONLY);

            // For files, use DeleteFileW
            if DeleteFileW(path_wide.as_ptr()) == 0 {
                let err = GetLastError();
                return Err(std::io::Error::from_raw_os_error(err as i32))
                    .with_context(|| format!("Failed to delete file: {}", path.display()));
            }
        }
    }

    Ok(())
}

/// Recursively delete directory contents (using long paths)
#[cfg(target_os = "windows")]
unsafe fn remove_directory_recursive(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        DeleteFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY,
        FILE_ATTRIBUTE_REPARSE_POINT, FindClose, FindFirstFileW, FindNextFileW, RemoveDirectoryW,
        SetFileAttributesW, WIN32_FIND_DATAW,
    };

    // Build search pattern: path\*
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
        // Directory is empty or error occurred, return success
        return Ok(());
    }

    loop {
        // Skip . and ..
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
                // Recursively delete subdirectory
                unsafe {
                    remove_directory_recursive(&item_path)?;
                }
                // Remove read-only attribute
                unsafe {
                    SetFileAttributesW(
                        item_wide.as_ptr(),
                        find_data.dwFileAttributes & !FILE_ATTRIBUTE_READONLY,
                    );
                }
                // Delete empty directory
                if unsafe { RemoveDirectoryW(item_wide.as_ptr()) } == 0 {
                    let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                    unsafe {
                        FindClose(find_handle);
                    }
                    return Err(std::io::Error::from_raw_os_error(err as i32)).with_context(|| {
                        format!("Failed to delete directory: {}", item_path.display())
                    });
                }
            } else {
                // Remove read-only attribute
                unsafe {
                    SetFileAttributesW(
                        item_wide.as_ptr(),
                        find_data.dwFileAttributes & !FILE_ATTRIBUTE_READONLY,
                    );
                }
                // Delete file or symlink
                if unsafe { DeleteFileW(item_wide.as_ptr()) } == 0 {
                    let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                    unsafe {
                        FindClose(find_handle);
                    }
                    return Err(std::io::Error::from_raw_os_error(err as i32)).with_context(|| {
                        format!("Failed to delete file: {}", item_path.display())
                    });
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

/// Convert path to Windows long path format
#[cfg(target_os = "windows")]
fn to_long_path(path: &Path) -> Result<PathBuf> {
    // Try to get absolute path, fall back to original if failed
    let absolute = match fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => {
            // If canonicalize fails (possibly due to long path), use absolute path
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

    // Check if already a UNC path
    let path_str = absolute.to_string_lossy().to_string();
    let has_prefix = path_str.starts_with(r"\\?\") || path_str.starts_with(r"\\?\UNC\");

    if has_prefix {
        return Ok(absolute);
    }

    // Add \\?\ prefix
    let long_path = if let Some(stripped) = path_str.strip_prefix(r"\\") {
        // UNC path: \\?\UNC\server\share
        PathBuf::from(format!(r"\\?\UNC\{stripped}"))
    } else {
        // Regular path: \\?\C:\path
        PathBuf::from(format!(r"\\?\{path_str}"))
    };

    Ok(long_path)
}

/// Recursively remove read-only attributes from directory and its contents
#[cfg(target_os = "windows")]
fn remove_readonly_recursively(path: &Path) -> Result<()> {
    let metadata = path
        .symlink_metadata()
        .with_context(|| format!("Failed to get path metadata: {}", path.display()))?;

    // Only process files and directories, skip symlinks
    if !metadata.file_type().is_symlink() {
        #[allow(clippy::permissions_set_readonly_false)]
        {
            let mut perms = metadata.permissions();
            perms.set_readonly(false);
            fs::set_permissions(path, perms)
                .with_context(|| format!("Failed to set permissions: {}", path.display()))?;
        }

        if metadata.is_dir() {
            for entry in fs::read_dir(path)
                .with_context(|| format!("Failed to read directory: {}", path.display()))?
            {
                let entry = entry.with_context(|| {
                    format!("Failed to read directory entry: {}", path.display())
                })?;
                remove_readonly_recursively(&entry.path())?;
            }
        }
    }
    Ok(())
}

/// Determine if an IO error is retryable (Windows-specific)
/// Retries are triggered only for:
/// - PermissionDenied
/// - Windows error code 32 (ERROR_SHARING_VIOLATION)
/// - Windows error code 33 (ERROR_LOCK_VIOLATION)
/// - Windows error code 5 (ERROR_ACCESS_DENIED)
#[cfg(target_os = "windows")]
fn is_retryable_error(e: &std::io::Error) -> bool {
    match e.kind() {
        std::io::ErrorKind::PermissionDenied => true,
        _ => {
            let os_code = e.raw_os_error();
            matches!(os_code, Some(5) | Some(32) | Some(33))
        }
    }
}

/// File deletion with exponential backoff retry
#[cfg(target_os = "windows")]
fn remove_with_retry(file: &FileInfo, anyway: bool) -> Result<()> {
    use crate::core::process::{find_processes_by_file, kill_process_force};
    use std::thread;
    use std::time::Duration;

    let mut wait_ms = RETRY_INITIAL_WAIT_MS;

    for attempt in 0..=RETRY_MAX_ATTEMPTS {
        match remove_entry(file) {
            Ok(()) => return Ok(()),
            Err(err) => {
                let io_err = err.downcast_ref::<std::io::Error>();
                let should_retry = io_err.is_some_and(is_retryable_error);

                if !should_retry || attempt == RETRY_MAX_ATTEMPTS {
                    return Err(err.context(format!(
                        "Deletion failed (after {} retries): {}",
                        attempt,
                        file.path.display()
                    )));
                }

                eprintln!(
                    "  Retrying ({}/{})... file may be in use: {}",
                    attempt + 1,
                    RETRY_MAX_ATTEMPTS,
                    file.path.display()
                );

                if anyway {
                    if let Ok(pids) = find_processes_by_file(&file.path) {
                        for pid in pids {
                            let _ = kill_process_force(pid);
                        }
                    }
                }

                thread::sleep(Duration::from_millis(wait_ms));
                wait_ms = (wait_ms * 2).min(RETRY_MAX_WAIT_MS);
            }
        }
    }

    unreachable!()
}

#[cfg(not(target_os = "windows"))]
fn remove_with_retry(file: &FileInfo, _anyway: bool) -> Result<()> {
    remove_entry(file)
}

fn remove_entry(file: &FileInfo) -> Result<()> {
    // On Windows, symlinks require special handling
    #[cfg(target_os = "windows")]
    {
        if file.is_symlink {
            // For symlinks, always use remove_file
            // This deletes the link itself, not the target
            match fs::remove_file(&file.path) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    // If it fails, try Windows-specific methods
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        // Try to get file attributes and remove read-only
                        if let Ok(metadata) = file.path.metadata() {
                            #[allow(clippy::permissions_set_readonly_false)]
                            {
                                let mut attrs = metadata.permissions();
                                attrs.set_readonly(false);
                                if let Err(_) = fs::set_permissions(&file.path, attrs) {
                                    // If unable to modify permissions, continue trying to delete
                                }
                            }
                        }
                        // Try deleting again
                        return fs::remove_file(&file.path).with_context(|| {
                            format!("Failed to delete symlink: {}", file.path.display())
                        });
                    }
                    return Err(e.into());
                }
            }
        }
    }

    // Regular handling for non-symlinks
    let result = if file.is_symlink {
        fs::remove_file(&file.path)
    } else if file.is_dir {
        // For directories, try remove_dir first (empty directory)
        match fs::remove_dir(&file.path) {
            Ok(_) => Ok(()),
            Err(e) => {
                // If it's a permission error, try modifying permissions then delete
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    // Recursively modify permissions of directory and its contents
                    #[cfg(target_os = "windows")]
                    if let Err(_) = remove_readonly_recursively(&file.path) {
                        // If unable to modify permissions, continue trying to delete
                    }
                    // Try deleting again
                    fs::remove_dir_all(&file.path)
                } else {
                    Err(e)
                }
            }
        }
    } else {
        fs::remove_file(&file.path)
    };

    result.with_context(|| format!("Deletion failed: {}", file.path.display()))
}
