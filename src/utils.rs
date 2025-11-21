use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::Path;

// Windows API相关导入
use windows_sys::Win32::Foundation::{GetLastError};
use windows_sys::Win32::Storage::FileSystem::{GetShortPathNameW};

// Windows MAX_PATH常量定义
const MAX_PATH: u32 = 260;

/// 将路径转换为Windows 8.3短文件名格式
/// 如果路径不包含空格或转换失败，则返回原始路径
pub fn get_short_path<P: AsRef<Path>>(path: P) -> Result<String, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    
    // 检查路径是否包含空格，如果没有则直接返回原始路径
    let path_str = path.to_string_lossy();
    if !path_str.contains(' ') {
        return Ok(path_str.to_string());
    }
    
    // 转换为Windows宽字符
    let os_str = OsStr::new(path);
    let wide_chars: Vec<u16> = os_str.encode_wide().chain(Some(0)).collect();
    
    // 分配缓冲区
    let mut short_path_buffer: Vec<u16> = vec![0; MAX_PATH as usize];
    let mut buffer_size = short_path_buffer.len() as u32;
    
    // 调用Windows API获取短路径名
    let result = unsafe {
        GetShortPathNameW(
            wide_chars.as_ptr(),
            short_path_buffer.as_mut_ptr(),
            buffer_size,
        )
    };
    
    // 检查结果
    if result == 0 {
        let error = unsafe { GetLastError() };
        return Err(format!("Failed to get short path: Win32 error {}", error).into());
    }
    
    // 如果结果大于缓冲区大小，需要更大的缓冲区
    if result > buffer_size {
        buffer_size = result;
        short_path_buffer.resize(buffer_size as usize, 0);
        
        let result = unsafe {
            GetShortPathNameW(
                wide_chars.as_ptr(),
                short_path_buffer.as_mut_ptr(),
                buffer_size,
            )
        };
        
        if result == 0 || result > buffer_size {
            let error = unsafe { GetLastError() };
            return Err(format!("Failed to get short path with larger buffer: Win32 error {}", error).into());
        }
    }
    
    // 找到字符串结束位置
    let end_pos = short_path_buffer.iter().position(|&c| c == 0).unwrap_or(short_path_buffer.len());
    
    // 转换回字符串
    let os_string = OsString::from_wide(&short_path_buffer[0..end_pos]);
    let short_path = os_string.to_string_lossy().to_string();
    
    Ok(short_path)
}