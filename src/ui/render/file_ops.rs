use crate::core::fs_ops::FileInfo;
use crate::core::process::FileLockInfo;
use crate::ui::Theme;
use anyhow::Result;
use inquire::Confirm;
use std::path::PathBuf;

use super::{format_size, tree_branches, truncate_string};

/// Display file/directory lock status
pub fn display_file_locks(infos: &[FileLockInfo]) {
    let theme = Theme::new();

    if infos.is_empty() {
        println!("{}", theme.warn("No paths found to check"));
        return;
    }

    println!("{} {}", theme.icon_search(), theme.title("File Lock Query"));
    println!();

    let total = infos.len();
    for (index, info) in infos.iter().enumerate() {
        let (branch, continuation) = tree_branches(total, index);

        let kind = if info.path.is_dir() {
            theme.blue("Dir")
        } else {
            theme.success("File")
        };

        let status = if info.locked {
            theme.error("Locked")
        } else {
            theme.success("Free")
        };

        println!(
            "{branch} {} {} {}",
            theme.highlight(info.path.display().to_string()),
            kind,
            status
        );

        if info.processes.is_empty() {
            if info.locked {
                println!(
                    "{continuation}└─ {}",
                    theme.warn("No locking process found, may need admin privileges or handle.exe")
                );
            }
        } else {
            let proc_total = info.processes.len();
            for (proc_index, proc_info) in info.processes.iter().enumerate() {
                let proc_last = proc_index == proc_total - 1;
                let proc_branch = if proc_last { "└─" } else { "├─" };
                let proc_continuation = if proc_last { "   " } else { "│  " };

                println!(
                    "{continuation}{proc_branch} {} {} ({})",
                    theme.info("Process"),
                    theme.success(&proc_info.name),
                    theme.muted(format!("PID: {}", proc_info.pid))
                );

                if !proc_info.cmd.is_empty() {
                    println!(
                        "{continuation}{proc_continuation} {} {}",
                        theme.info("Command"),
                        theme.muted(truncate_string(&proc_info.cmd, 80))
                    );
                }
            }
        }

        if continuation == "│  " {
            println!("{continuation}");
        }
    }
}

/// Display deletion preview
pub fn display_deletion_preview(files: &[FileInfo]) {
    let theme = Theme::new();
    let total_size: u64 = files.iter().map(|f| f.size).sum();
    let (file_count, dir_count) = files.iter().fold((0, 0), |(files, dirs), f| {
        if f.is_dir {
            (files, dirs + 1)
        } else {
            (files + 1, dirs)
        }
    });

    println!(
        "{} {} {} {}",
        theme.title("Summary:"),
        theme.success(format!("{file_count} files")),
        theme.blue(format!("{dir_count} directories")),
        theme.warn(format!("Total size: {}", format_size(total_size)))
    );
    println!();

    let total = files.len().min(10);
    for file in files.iter().take(total) {
        let icon = if file.is_dir {
            theme.icon_folder()
        } else if file.is_symlink {
            theme.icon_link()
        } else {
            theme.icon_file()
        };

        let size_str = if !file.is_dir && !file.is_symlink {
            let size = format!(" ({})", format_size(file.size));
            theme.muted(size)
        } else {
            String::new()
        };

        let file_type = if file.is_dir {
            theme.blue("Dir")
        } else if file.is_symlink {
            theme.accent("Symlink")
        } else {
            theme.success("File")
        };

        println!(
            "  {} {} {}{}",
            icon,
            file.path.display(),
            file_type,
            size_str
        );
    }

    if files.len() > 10 {
        println!(
            "{}",
            theme.muted(format!("  ... {} more items", files.len() - 10))
        );
    }

    println!();
}

/// Confirm deletion operation
pub fn confirm_deletion(files: &[FileInfo], skip_confirm: bool, dry_run: bool) -> Result<bool> {
    let theme = Theme::new();

    if dry_run {
        println!(
            "{} {}",
            theme.icon_search(),
            theme.info_bold("Preview mode - no files will be deleted")
        );
        display_deletion_preview(files);
        return Ok(true);
    }

    if skip_confirm {
        return Ok(true);
    }

    println!(
        "{} {}",
        theme.icon_warning(),
        theme.error_bold("About to delete the following")
    );
    display_deletion_preview(files);

    let confirm = Confirm::new("Confirm deleting these items? This cannot be undone!")
        .with_default(false)
        .with_help_message("Use --force to skip this confirmation")
        .prompt()?;

    Ok(confirm)
}

/// Check file locks and handle them
///
/// - Default: show lock info and ask user whether to continue
/// - `--anyway`: auto-kill locking processes, then continue
///
/// Returns true to proceed with deletion, false to cancel
pub fn check_and_warn_file_locks(files: &[FileInfo], anyway: bool) -> Result<bool> {
    use crate::core::process::{inspect_file_locks, kill_processes_force};

    let theme = Theme::new();

    let paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();

    let lock_infos = match inspect_file_locks(&paths) {
        Ok(infos) => infos,
        Err(e) => {
            eprintln!(
                "{} {}: {}",
                theme.icon_warning(),
                theme.warn("Unable to check file locks"),
                e
            );
            eprintln!("{}", theme.muted("Will proceed with deletion"));
            return Ok(true);
        }
    };

    let locked_files: Vec<FileLockInfo> = lock_infos
        .into_iter()
        .filter(|info| info.locked || !info.processes.is_empty())
        .collect();

    if locked_files.is_empty() {
        return Ok(true);
    }

    if anyway {
        let mut pids = Vec::new();
        for info in &locked_files {
            for proc in &info.processes {
                if !pids.contains(&proc.pid) {
                    pids.push(proc.pid);
                }
            }
        }

        println!();
        println!(
            "{} {}",
            theme.icon_warning(),
            theme.error_bold("Files locked, killing locking processes...")
        );
        println!();
        display_file_locks(&locked_files);
        println!();

        let results = kill_processes_force(&pids);

        for (pid, result) in results {
            match result {
                Ok(_) => {
                    println!(
                        "{} {}",
                        theme.icon_success(),
                        theme.muted(format!("Killed process PID: {pid}"))
                    );
                }
                Err(e) => {
                    println!(
                        "{} {}",
                        theme.icon_error(),
                        theme.error(format!("Failed to kill process PID {pid}: {e}"))
                    );
                }
            }
        }
        println!();

        return Ok(true);
    }

    println!();
    println!(
        "{} {}",
        theme.icon_warning(),
        theme.error_bold("Files are locked")
    );
    println!();

    display_file_locks(&locked_files);

    println!();

    let confirm = Confirm::new("These files are in use, continue trying to delete?")
        .with_default(false)
        .with_help_message("Use --anyway to auto-kill locking processes and delete")
        .prompt()?;

    Ok(confirm)
}

/// Display deletion results
pub fn display_removal_results(
    results: &[(std::path::PathBuf, Result<()>)],
    dry_run: bool,
    verbose: bool,
) {
    let theme = Theme::new();
    let action = if dry_run { "Preview" } else { "Delete" };
    let (success_count, error_count) =
        results
            .iter()
            .fold((0, 0), |(success, error), (_, result)| {
                if result.is_ok() {
                    (success + 1, error)
                } else {
                    (success, error + 1)
                }
            });

    if !verbose {
        println!(
            "{} {} {}",
            theme.title("Done"),
            theme.success(format!("Success: {success_count}")),
            theme.error(format!("Failed: {error_count}"))
        );

        if error_count > 0 {
            for (path, result) in results {
                if let Err(e) = result {
                    println!(
                        "{} {} {}",
                        theme.icon_error(),
                        theme.error(format!("Failed to delete {}", path.display())),
                        e
                    );
                }
            }
        }
        return;
    }

    println!(
        "{} {} {}",
        theme.title("Done"),
        theme.success(format!("Success: {success_count}")),
        theme.error(format!("Failed: {error_count}"))
    );

    for (path, result) in results {
        match result {
            Ok(()) => println!(
                "{} {}",
                theme.icon_success(),
                theme.muted(format!("{} {}", action, path.display()))
            ),
            Err(e) => println!(
                "{} {} {}",
                theme.icon_error(),
                theme.error(format!("Failed to delete {}", path.display())),
                e
            ),
        }
    }
}
