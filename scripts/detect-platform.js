/**
 * 检测当前操作系统和架构
 */

function detectPlatform() {
  const platform = process.platform;
  const arch = process.arch;
  
  // 平台映射
  const platformMap = {
    'win32': 'windows',
    'darwin': 'macos',
    'linux': 'linux',
  };
  
  // 架构映射
  const archMap = {
    'x64': 'x86_64',
    'arm64': 'aarch64',
  };
  
  const mappedPlatform = platformMap[platform];
  const mappedArch = archMap[arch];
  
  if (!mappedPlatform) {
    throw new Error(`不支持的操作系统: ${platform}`);
  }
  
  if (!mappedArch) {
    throw new Error(`不支持的架构: ${arch}`);
  }
  
  return {
    platform: mappedPlatform,
    arch: mappedArch,
    raw: {
      platform,
      arch,
    },
  };
}

function getBinaryName(platformInfo) {
  const { platform, arch } = platformInfo;

  // 架构名称映射：将 x86_64 映射为 x64 以匹配现有构建包命名
  const archNameMap = {
    'x86_64': 'x64',
    'aarch64': 'aarch64' // 保持不变
  };

  const mappedArch = archNameMap[arch] || arch;

  // 生成 zip 包文件名，匹配现有构建包命名
  // 例如: linux-aarch64.zip, windows-x64.zip, macos-x64.zip
  let binaryName = `${platform}-${mappedArch}.zip`;

  return binaryName;
}

function getDownloadUrl(version, platformInfo) {
  const binaryName = getBinaryName(platformInfo);
  const repo = 'Protagonistss/ziro';
  
  // GitHub Release 下载 URL
  return `https://github.com/${repo}/releases/download/v${version}/${binaryName}`;
}

module.exports = {
  detectPlatform,
  getBinaryName,
  getDownloadUrl,
};

