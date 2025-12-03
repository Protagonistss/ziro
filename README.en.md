# Ziro

<div align="center">

A fast, cross-platform port management tool

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/ziro.svg)](https://crates.io/crates/ziro)
[![npm](https://img.shields.io/npm/v/@protagonistss/ziro.svg)](https://www.npmjs.com/package/@protagonistss/ziro)

[English](README.en.md) | [简体中文](README.zh.md)

</div>

## Introduction

Ziro is a powerful command-line tool for quickly finding and managing processes that occupy ports. Supports Windows, Linux, and macOS platforms.

### Core Features

- 🔍 **Quick Search** - Instantly find processes occupying specified ports
- 🎯 **Batch Kill** - Support terminating processes on multiple ports simultaneously
- 📊 **Detailed Information** - Display process PID, name, command, CPU and memory usage
- 🎨 **Beautiful Interface** - Colored output and table display for better visual experience
- 💬 **Interactive Selection** - Interactive selection and confirmation before terminating processes
- 🌍 **Cross-Platform** - Supports Windows, Linux, and macOS

## Installation

### Using Cargo (Rust Users)

```bash
cargo install ziro
```

### Using npm (Node.js Users)

```bash
npm install -g @protagonistss/ziro
```

Or use other package managers:

```bash
# Using yarn
yarn global add @protagonistss/ziro

# Using pnpm
pnpm add -g @protagonistss/ziro
```

## Usage

### Find Process Occupying a Port

```bash
# Find process occupying port 8080
ziro find 8080
```

Output example:
```
Found process occupying port:
  Port: 8080
  PID: 12345
  Name: node
  Command: node server.js
  CPU: 2.3%
  Memory: 128 MB
```

### Kill Process Occupying a Port

```bash
# Kill process occupying port 8080
ziro kill 8080

# Kill processes on multiple ports
ziro kill 8080 3000 5000
```

The program will display all found processes, allowing you to interactively select which processes to terminate and confirm before termination.

### List All Port Occupancy

```bash
ziro list
```

Output example:
```
Current port occupancy:
╭──────┬───────┬──────────┬─────────────────────────┬───────┬────────╮
│ Port │  PID  │   Name   │        Command          │  CPU  │ Memory │
├──────┼───────┼──────────┼─────────────────────────┼───────┼────────┤
│ 3000 │ 12345 │ node     │ node app.js             │ 1.2%  │ 95 MB  │
│ 8080 │ 23456 │ python   │ python -m http.server   │ 0.5%  │ 45 MB  │
│ 5432 │ 34567 │ postgres │ /usr/bin/postgres       │ 3.1%  │ 256 MB │
╰──────┴───────┴──────────┴─────────────────────────┴───────┴────────╯
```

## Command Reference

```
Ziro - Cross-platform port management tool

Usage:
  ziro <COMMAND>

Commands:
  find <PORT>          Find process occupying specified port
  kill <PORT>...       Kill processes occupying specified ports (multiple allowed)
  list                 List all port occupancy
  help                 Show help information

Options:
  -h, --help           Show help information
  -V, --version        Show version information
```

## Platform Support

| Operating System | Architecture | Status |
|-----------------|--------------|--------|
| Windows         | x64          | ✅ Fully Supported |
| Linux           | x64          | ✅ Fully Supported |
| Linux           | arm64        | ✅ Fully Supported |
| macOS           | x64          | ✅ Fully Supported |
| macOS           | arm64        | ✅ Fully Supported |

## Tech Stack

- **Core Language**: Rust
- **CLI Parsing**: clap
- **System Info**: sysinfo
- **Interactive UI**: inquire
- **Table Display**: tabled
- **Colored Output**: colored

## Development

### Build Project

```bash
# Clone repository
git clone https://github.com/Protagonistss/ziro.git
cd ziro

# Build
cargo build --release

# Run
cargo run -- find 8080
```

### Run Tests

```bash
cargo test
```

### Code Formatting

```bash
cargo fmt
```

### Code Linting

```bash
cargo clippy
```

## Contributing

Contributions are welcome! Feel free to submit Issues or Pull Requests.

### Contribution Guidelines

1. Fork this project
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Create a Pull Request

## License

This project is open source under the [MIT License](LICENSE).

## Acknowledgments

Thanks to all contributors and users for their support!

## Related Projects

- [fkill](https://github.com/sindresorhus/fkill) - Node.js version of process termination tool
- [lsof](https://github.com/lsof-org/lsof) - Unix system file and network connection viewing tool

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.

---

<div align="center">
  
**If this project helps you, please give it a ⭐️**

Made with ❤️ by [huangshan](https://github.com/Protagonistss)

</div>

