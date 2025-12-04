#!/usr/bin/env node

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const { detectPlatform, getBinaryName, getDownloadUrl } = require('./detect-platform');

// 从 package.json 读取版本号
const packageJson = require('../package.json');
const version = packageJson.version;

async function download(url, dest) {
  return new Promise((resolve, reject) => {
    console.log(`下载: ${url}`);
    
    const file = fs.createWriteStream(dest);
    
    https.get(url, {
      headers: {
        'User-Agent': 'ziro-installer',
      },
    }, (response) => {
      // 处理重定向
      if (response.statusCode === 302 || response.statusCode === 301) {
        const redirectUrl = response.headers.location;
        console.log(`重定向到: ${redirectUrl}`);
        
        https.get(redirectUrl, (redirectResponse) => {
          if (redirectResponse.statusCode !== 200) {
            reject(new Error(`下载失败，HTTP 状态码: ${redirectResponse.statusCode}`));
            return;
          }
          
          const totalBytes = parseInt(redirectResponse.headers['content-length'], 10);
          let downloadedBytes = 0;
          
          redirectResponse.on('data', (chunk) => {
            downloadedBytes += chunk.length;
            const percent = ((downloadedBytes / totalBytes) * 100).toFixed(1);
            process.stdout.write(`\r下载进度: ${percent}%`);
          });
          
          redirectResponse.pipe(file);
          
          file.on('finish', () => {
            file.close();
            console.log('\n下载完成');
            resolve();
          });
        }).on('error', (err) => {
          fs.unlink(dest, () => {});
          reject(err);
        });
        
        return;
      }
      
      if (response.statusCode !== 200) {
        reject(new Error(`下载失败，HTTP 状态码: ${response.statusCode}`));
        return;
      }
      
      const totalBytes = parseInt(response.headers['content-length'], 10);
      let downloadedBytes = 0;
      
      response.on('data', (chunk) => {
        downloadedBytes += chunk.length;
        if (totalBytes) {
          const percent = ((downloadedBytes / totalBytes) * 100).toFixed(1);
          process.stdout.write(`\r下载进度: ${percent}%`);
        }
      });
      
      response.pipe(file);
      
      file.on('finish', () => {
        file.close();
        console.log('\n下载完成');
        resolve();
      });
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

async function extractZip(zipPath, extractDir) {
  return new Promise((resolve, reject) => {
    console.log(`解压: ${zipPath}`);

    try {
      // 使用系统内置的 unzip 命令（跨平台）
      if (process.platform === 'win32') {
        // Windows: 使用 PowerShell 的 Expand-Archive
        execSync(`powershell -Command "Expand-Archive -Path '${zipPath}' -DestinationPath '${extractDir}' -Force"`, { stdio: 'inherit' });
      } else {
        // Unix-like systems: 使用 unzip 命令
        execSync(`unzip -o '${zipPath}' -d '${extractDir}'`, { stdio: 'inherit' });
      }

      console.log('解压完成');
      resolve();
    } catch (error) {
      reject(new Error(`解压失败: ${error.message}`));
    }
  });
}

async function install() {
  try {
    console.log('正在安装 ziro...');
    
    // 检测平台
    const platformInfo = detectPlatform();
    console.log(`检测到平台: ${platformInfo.platform} (${platformInfo.arch})`);
    
    // 确保 bin 目录存在
    const binDir = path.join(__dirname, '..', 'bin');
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }
    
    // 生成二进制文件名和路径
    const binaryName = getBinaryName(platformInfo);
    const zipPath = path.join(binDir, binaryName);
    const binaryPath = path.join(binDir, platformInfo.raw.platform === 'win32' ? 'ziro.exe' : 'ziro');

    // 构建下载 URL
    const downloadUrl = getDownloadUrl(version, platformInfo);

    // 下载 zip 文件
    try {
      await download(downloadUrl, zipPath);

      // 解压 zip 文件
      await extractZip(zipPath, binDir);

      // 删除 zip 文件
      fs.unlinkSync(zipPath);

      // 检查二进制文件是否存在，如果存在解压后的原始文件，重命名
      const extractedBinaryPath = path.join(binDir, 'ziro');
      if (fs.existsSync(extractedBinaryPath) && platformInfo.raw.platform === 'win32') {
        // Windows下需要.exe扩展名
        const targetPath = path.join(binDir, 'ziro.exe');
        if (fs.existsSync(targetPath)) {
          fs.unlinkSync(targetPath);
        }
        fs.renameSync(extractedBinaryPath, targetPath);
      }

      // 再次检查二进制文件是否存在
      if (!fs.existsSync(binaryPath)) {
        throw new Error('解压后未找到二进制文件');
      }

    } catch (error) {
      console.error('\n安装失败:', error.message);
      console.error('\n可能的原因:');
      console.error('1. 该版本的预编译二进制文件尚未发布');
      console.error('2. 网络连接问题');
      console.error('3. 当前平台/架构不受支持');
      console.error('\n替代方案:');
      console.error('1. 使用 Cargo 安装: cargo install ziro');
      console.error('2. 从源码编译: git clone https://github.com/Protagonistss/ziro && cd ziro && cargo build --release');
      process.exit(1);
    }
    
    // 设置执行权限（Unix 系统）
    if (platformInfo.raw.platform !== 'win32') {
      fs.chmodSync(binaryPath, 0o755);
      console.log('设置执行权限完成');
    }
    
    console.log('✓ 安装成功！');
    console.log('\n使用方法:');
    console.log('  ziro find <port>     - 查找占用端口的进程');
    console.log('  ziro kill <port>...  - 终止占用端口的进程');
    console.log('  ziro list            - 列出所有端口占用情况');
    console.log('  ziro --help          - 查看帮助信息');
    
  } catch (error) {
    console.error('安装失败:', error.message);
    console.error('\n如果问题持续存在，请访问: https://github.com/Protagonistss/ziro/issues');
    process.exit(1);
  }
}

// 仅在直接运行时执行（不是被 require 时）
if (require.main === module) {
  install();
}

module.exports = { install };

