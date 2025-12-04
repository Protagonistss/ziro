#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// 获取二进制文件路径
function getBinaryPath() {
  const platform = process.platform;

  let binaryName = 'ziro';
  if (platform === 'win32') {
    binaryName = 'ziro.exe';
  }

  const binaryPath = path.join(__dirname, binaryName);

  if (!fs.existsSync(binaryPath)) {
    console.error('错误: 找不到 ziro 二进制文件');
    console.error('请尝试重新安装: npm install -g ziro');
    process.exit(1);
  }

  return binaryPath;
}

// 执行二进制文件
function run() {
  const binaryPath = getBinaryPath();
  const args = process.argv.slice(2);
  
  const child = spawn(binaryPath, args, {
    stdio: 'inherit',
    windowsHide: false,
  });
  
  child.on('error', (error) => {
    console.error('执行失败:', error.message);
    process.exit(1);
  });
  
  child.on('exit', (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
    } else {
      process.exit(code || 0);
    }
  });
}

run();

