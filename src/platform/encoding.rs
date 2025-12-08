// Windows 控制台 UTF-8 初始化
#[cfg(target_os = "windows")]
pub fn init_windows_console() {
    use winapi::um::wincon::{SetConsoleCP, SetConsoleOutputCP};
    unsafe {
        // 设置输入输出编码为 UTF-8，减少乱码
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
    }
}

// 非 Windows 平台无需处理
#[cfg(not(target_os = "windows"))]
pub fn init_windows_console() {}
