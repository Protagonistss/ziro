/// 编码转换模块
/// 处理命令输出的编码转换问题
///
/// 安全地转换命令输出为字符串，尝试多种编码方式
pub fn safe_command_output_to_string(stdout: &[u8]) -> String {
    // 首先尝试 UTF-8
    if let Ok(text) = std::str::from_utf8(stdout) {
        return text.to_string();
    }

    // 如果 UTF-8 失败，尝试检测 Windows 代码页
    #[cfg(target_os = "windows")]
    {
        // 尝试常见的中文编码
        if let Some(text) = try_decode_as_gbk(stdout) {
            return text;
        }

        // 尝试 Windows-1252
        if let Some(text) = try_decode_as_windows_1252(stdout) {
            return text;
        }
    }

    // 最后的回退：使用 lossy 转换，但记录错误
    let lossy = String::from_utf8_lossy(stdout);
    if lossy.contains('\u{FFFD}') {
        // 如果有替换字符，说明有编码问题，但这在系统命令输出中很常见
        // 记录到 stderr 而不是 stdout，避免干扰程序输出
        eprintln!("警告: 命令输出包含非 UTF-8 字符，可能影响显示效果");
    }
    lossy.to_string()
}

#[cfg(target_os = "windows")]
fn try_decode_as_gbk(data: &[u8]) -> Option<String> {
    // 简化的 GBK 检测和转换
    // 这是一个基本实现，实际项目中可能需要使用 encoding crate
    if data.len() >= 2 {
        // 检查是否可能是 GBK 编码
        let mut valid_gbk = true;
        let mut i = 0;
        while i < data.len() - 1 {
            if data[i] >= 0x81 && data[i] <= 0xFE && data[i + 1] >= 0x40 && data[i + 1] <= 0xFE {
                // 可能是 GBK 字符
                i += 2;
            } else if data[i] <= 0x7F {
                // ASCII 字符
                i += 1
            } else {
                valid_gbk = false;
                break;
            }
        }

        if valid_gbk {
            // 简单的 GBK 到 UTF-8 转换占位符
            // 实际实现需要使用适当的编码库
            return Some(format!("[GBK编码数据，长度: {}]", data.len()));
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn try_decode_as_windows_1252(data: &[u8]) -> Option<String> {
    // Windows-1252 检测
    // 检查是否包含有效的 Windows-1252 字符
    for &byte in data {
        if byte == 0x81 || byte == 0x8D || byte == 0x8F || byte == 0x90 || byte == 0x9D {
            // 这些是 Windows-1252 中的控制字符，在 UTF-8 中无效
            return Some(format!("[Windows-1252编码数据，长度: {}]", data.len()));
        }
    }
    None
}
