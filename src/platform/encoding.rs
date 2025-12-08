// Windows 控制台 UTF-8 初始化
#[cfg(target_os = "windows")]
pub fn init_windows_console() {
    use winapi::um::wincon::{SetConsoleCP, SetConsoleOutputCP};
    unsafe {
        // 设置输入输出编码为 UTF-8，减少乱码
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
    }

    enable_virtual_terminal_processing();
}

// 非 Windows 平台无需处理
#[cfg(not(target_os = "windows"))]
pub fn init_windows_console() {}

/// 启用 Windows 控制台的虚拟终端序列，确保光标移动/清屏等 ANSI 序列生效
#[cfg(target_os = "windows")]
fn enable_virtual_terminal_processing() {
    use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::{STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};
    use winapi::um::wincon::ENABLE_VIRTUAL_TERMINAL_PROCESSING;

    unsafe {
        for handle_id in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let handle = GetStdHandle(handle_id);
            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                continue;
            }

            let mut mode: u32 = 0;
            if GetConsoleMode(handle, &mut mode) == 0 {
                continue;
            }

            let new_mode = mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            if new_mode != mode {
                let _ = SetConsoleMode(handle, new_mode);
            }
        }
    }
}
