/// Encoding conversion module
/// Handles encoding conversion for command output
pub fn safe_command_output_to_string(stdout: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(stdout) {
        return text.to_string();
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(text) = try_decode_as_gbk(stdout) {
            return text;
        }

        if let Some(text) = try_decode_as_windows_1252(stdout) {
            return text;
        }
    }

    String::from_utf8_lossy(stdout).to_string()
}

#[cfg(target_os = "windows")]
fn try_decode_as_gbk(data: &[u8]) -> Option<String> {
    use encoding_rs::GBK;
    let (text, _, had_errors) = GBK.decode(data);
    if had_errors {
        return None;
    }
    let s = text.to_string();
    if s.contains('\u{FFFD}') {
        return None;
    }
    Some(s)
}

#[cfg(target_os = "windows")]
fn try_decode_as_windows_1252(data: &[u8]) -> Option<String> {
    use encoding_rs::WINDOWS_1252;
    let (text, _, had_errors) = WINDOWS_1252.decode(data);
    if had_errors {
        return None;
    }
    Some(text.to_string())
}
