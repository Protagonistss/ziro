# Usage Guide

### Find Process Occupying a Port

```bash
# Find process occupying port 8080
ziro find 8080
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

### Check File/Directory Lock

```bash
# Check a single file
ziro who C:\path\file.txt

# Check multiple paths
ziro who .\logs .\data\app.db
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
  who <PATH>...        Check processes occupying a file or directory
  help                 Show help information

Options:
  -h, --help           Show help information
  -V, --version        Show version information
```
