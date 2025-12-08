use anyhow::Result;
use clap::Parser;
use ziro::cli::{
    Cli, Commands, display_version, handle_find, handle_kill, handle_list, handle_remove,
    handle_top,
};
#[cfg(target_os = "windows")]
use ziro::platform::encoding;
use ziro::platform::term;
use ziro::ui;

fn main() {
    #[cfg(target_os = "windows")]
    encoding::init_windows_console();

    if let Err(e) = run() {
        ui::display_error(&e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let profile = term::detect_profile(&cli);
    term::apply_profile_env(&profile);
    term::set_global_profile(profile);

    if cli.version {
        display_version();
        return Ok(());
    }

    match cli.command {
        Some(Commands::Find { ports }) => handle_find(ports)?,
        Some(Commands::Kill { ports, force }) => handle_kill(ports, force)?,
        Some(Commands::List) => handle_list()?,
        Some(Commands::Remove {
            paths,
            force,
            recursive,
            dry_run,
            verbose,
            anyway,
        }) => handle_remove(paths, force, recursive, dry_run, verbose, anyway)?,
        Some(Commands::Top {
            interval,
            limit,
            cpu,
            cmd,
            once,
        }) => handle_top(interval, limit, cpu, cmd, once)?,
        None => println!("使用 'ziro --help' 查看可用命令"),
    }

    Ok(())
}
