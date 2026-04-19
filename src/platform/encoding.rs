// Windows console UTF-8 initialization
#[cfg(target_os = "windows")]
pub fn init_windows_console() {
    use windows_sys::Win32::System::Console::{SetConsoleCP, SetConsoleOutputCP};
    unsafe {
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
    }

    enable_virtual_terminal_processing();
}

// No-op for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub fn init_windows_console() {}

/// Enable Windows console virtual terminal sequences for cursor movement, screen clearing, etc.
#[cfg(target_os = "windows")]
fn enable_virtual_terminal_processing() {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_ERROR_HANDLE,
        STD_OUTPUT_HANDLE, SetConsoleMode,
    };

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
                // Best-effort VT enablement; ignore if unsupported
                let _ = SetConsoleMode(handle, new_mode);
            }
        }
    }
}
