use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::Path;

// Windows API相关导入
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

// Windows MAX_PATH常量定义
const MAX_PATH: u32 = 260;

// 全局调试控制标志，默认关闭
static mut DEBUG_MODE: bool = false;

/// 设置调试模式
pub fn set_debug_mode(enabled: bool) {
    unsafe {
        DEBUG_MODE = enabled;
    }
}

/// 获取当前调试模式状态
pub fn is_debug_mode() -> bool {
    unsafe { DEBUG_MODE }
}

/// 创建一个条件打印宏，只有在调试模式下才会打印
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        if $crate::is_debug_mode() {
            println!($($arg)*);
        }
    };
}

/// 将路径转换为Windows 8.3短文件名格式
/// 如果路径不包含空格或转换失败，则返回原始路径
pub fn get_short_path<P: AsRef<Path>>(path: P) -> Result<String, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    let path_str = path.to_string_lossy();
    debug_println!("[DEBUG utils] Starting get_short_path for: {}", path_str);

    // 检查路径是否包含空格，如果没有则直接返回原始路径
    debug_println!("[DEBUG utils] Checking if path contains spaces...");
    if !path_str.contains(' ') {
        debug_println!("[DEBUG utils] Path does not contain spaces, returning original path");
        return Ok(path_str.to_string());
    }
    debug_println!("[DEBUG utils] Path contains spaces, need to get short path");

    // 转换为Windows宽字符
    debug_println!("[DEBUG utils] Converting path to UTF-16 wide characters...");
    let os_str = OsStr::new(path);
    let wide_chars: Vec<u16> = os_str.encode_wide().chain(Some(0)).collect();
    debug_println!(
        "[DEBUG utils] UTF-16 conversion completed, length: {}",
        wide_chars.len()
    );

    // 分配缓冲区
    debug_println!(
        "[DEBUG utils] Creating buffer with initial size {}...",
        MAX_PATH
    );
    let mut short_path_buffer: Vec<u16> = vec![0; MAX_PATH as usize];
    let mut buffer_size = short_path_buffer.len() as u32;
    debug_println!("[DEBUG utils] Buffer created successfully");

    // 调用Windows API获取短路径名
    debug_println!("[DEBUG utils] Calling GetShortPathNameW API...");
    let result = unsafe {
        GetShortPathNameW(
            wide_chars.as_ptr(),
            short_path_buffer.as_mut_ptr(),
            buffer_size,
        )
    };
    debug_println!("[DEBUG utils] GetShortPathNameW result: {}", result);

    // 检查结果
    if result == 0 {
        let error = unsafe { GetLastError() };
        println!(
            "[ERROR utils] Failed to get short path: Win32 error {}",
            error
        );
        return Err(format!("Failed to get short path: Win32 error {}", error).into());
    }

    // 如果结果大于缓冲区大小，需要更大的缓冲区
    if result > buffer_size {
        buffer_size = result;
        debug_println!(
            "[DEBUG utils] Result > buffer size, need to resize buffer to {}",
            buffer_size
        );
        short_path_buffer.resize(buffer_size as usize, 0);
        debug_println!("[DEBUG utils] Buffer resized successfully");

        debug_println!("[DEBUG utils] Calling GetShortPathNameW API again with resized buffer...");
        let result = unsafe {
            GetShortPathNameW(
                wide_chars.as_ptr(),
                short_path_buffer.as_mut_ptr(),
                buffer_size,
            )
        };
        debug_println!("[DEBUG utils] Second GetShortPathNameW result: {}", result);

        if result == 0 || result > buffer_size {
            let error = unsafe { GetLastError() };
            println!(
                "[ERROR utils] Failed to get short path with larger buffer: Win32 error {}",
                error
            );
            return Err(format!(
                "Failed to get short path with larger buffer: Win32 error {}",
                error
            )
            .into());
        }
    }

    // 找到字符串结束位置
    debug_println!("[DEBUG utils] Finding null terminator in buffer...");
    let end_pos = short_path_buffer
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(short_path_buffer.len());
    debug_println!(
        "[DEBUG utils] Found null terminator at position {}",
        end_pos
    );

    // 转换回字符串
    debug_println!("[DEBUG utils] Converting UTF-16 buffer back to UTF-8 string...");
    let os_string = OsString::from_wide(&short_path_buffer[0..end_pos]);
    let short_path = os_string.to_string_lossy().to_string();
    debug_println!(
        "[DEBUG utils] Conversion completed, short path: {}",
        short_path
    );

    debug_println!("[DEBUG utils] get_short_path completed successfully");
    Ok(short_path)
}
