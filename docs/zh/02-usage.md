# 使用指南

### 查找占用端口的进程

```bash
# 查找占用 8080 端口的进程
ziro find 8080
```

输出示例：
```
找到占用端口的进程：
  端口: 8080
  PID: 12345
  名称: node
  命令: node server.js
  CPU: 2.3%
  内存: 128 MB
```

### 终止占用端口的进程

```bash
# 终止占用 8080 端口的进程
ziro kill 8080

# 终止多个端口的进程
ziro kill 8080 3000 5000
```

程序会显示找到的所有进程，让你交互式地选择要终止的进程，并在终止前进行确认。

### 列出所有端口占用情况

```bash
ziro list
```

### 查看文件/目录占用

```bash
# 查看单个文件
ziro who C:\path\file.txt

# 查看多个路径
ziro who .\logs .\data\app.db
```

## 命令参考

```
Ziro - 跨平台端口管理工具

使用方法:
  ziro <COMMAND>

命令:
  find <PORT>          查找占用指定端口的进程
  kill <PORT>...       终止占用指定端口的进程（可指定多个）
  list                 列出所有端口占用情况
  who <PATH>...        查找占用指定文件或目录的进程
  help                 显示帮助信息

选项:
  -h, --help           显示帮助信息
  -V, --version        显示版本信息
```
