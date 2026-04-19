# Development Guide

## Tech Stack

- **Core Language**: Rust
- **CLI Parsing**: clap
- **System Info**: sysinfo
- **Interactive UI**: inquire
- **Colored Output**: colored

## Project Structure
- Rust Core: Entry is `src/bin/ziro.rs`; `src/cli/` (args definition, handlers); `src/core/` (port scanning, process killing, fs_ops, top); `src/platform/` (term, encoding); `src/ui/` (render, theme, icons).
- Node Distribution: `bin/ziro.js` acts as npm startup proxy, `scripts/install.js` downloads platform binaries.

## Build and Test
- `cargo build --release`: Build optimized binary to `target/release/ziro`.
- `cargo run -- <command>`: Local debugging.
- `cargo test`: Run all unit/integration tests.
- `cargo fmt` & `cargo clippy -- -D warnings`: Formatting and static checking.

## Coding Style
- Rust 2024 Edition.
- Error handling primarily uses `anyhow::Result<T>`. UI logic is centralized in `src/ui/render.rs`.
