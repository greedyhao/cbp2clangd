use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Component, Path, PathBuf};

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

/// 逻辑上计算绝对路径（不解析符号链接或映射驱动器，保留盘符）
/// 替代 std::fs::canonicalize，避免将 Z: 解析为 UNC 路径
pub fn compute_absolute_path(path: &Path) -> std::io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    // 逻辑消除 ".." 和 "."
    let mut clean_path = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::ParentDir => {
                clean_path.pop();
            }
            Component::Normal(c) => {
                clean_path.push(c);
            }
            Component::RootDir => {
                // 在 Windows 上，push RootDir (例如 "\") 可能会重置 PathBuf
                // 但如果是遍历 Components 重组，我们需要小心处理
                // 通常 Prefix 包含了盘符，RootDir 包含了斜杠
                clean_path.push(Component::RootDir.as_os_str());
            }
            Component::Prefix(prefix) => {
                clean_path.push(Component::Prefix(prefix).as_os_str());
            }
            Component::CurDir => {}
        }
    }

    // 如果结果为空（例如在某些边缘情况下），至少返回 "."
    if clean_path.as_os_str().is_empty() {
        Ok(PathBuf::from("."))
    } else {
        Ok(clean_path)
    }
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

/// 辅助函数：如果路径包含空格，则用引号包裹
pub fn quote_if_needed(path: &str) -> String {
    if path.contains(' ') {
        format!("\"{}\"", path)
    } else {
        path.to_string()
    }
}

/// 辅助函数：转义Ninja语法中的特殊字符（空格和冒号）
pub fn escape_ninja_path(path: &str) -> String {
    // Ninja 中空格转义为 $ ，冒号转义为 $:
    path.replace(" ", "$ ").replace(":", "$:")
}

/// 辅助函数：逻辑上解析绝对路径（不依赖文件系统存在性，仅处理路径组件）
/// 用于解决 project_dir + ../../file.c 的路径计算
pub fn get_clean_absolute_path(base: &Path, rel: &Path) -> PathBuf {
    let mut result = base.to_path_buf();

    // 处理Windows绝对路径的特殊情况
    // 当遇到完整的Windows绝对路径（盘符+根目录）时，需要正确组合
    let mut components = rel.components();
    while let Some(component) = components.next() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(c) => {
                result.push(c);
            }
            Component::RootDir => {
                // 如果遇到根目录（如Linux的 / 或 Windows 的 \），重置路径
                result = PathBuf::from(component.as_os_str());
            }
            Component::Prefix(prefix) => {
                 // Windows 盘符，先设置盘符
                 result = PathBuf::from(prefix.as_os_str());
                 // 检查下一个组件是否是根目录（\）
                 if let Some(next_component) = components.next() {
                     if let Component::RootDir = next_component {
                         // 如果是，将根目录添加到盘符后面
                         result.push(next_component);
                     } else {
                         // 否则，重新处理这个组件
                         result.push(next_component);
                     }
                 }
            }
            Component::CurDir => {}
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_debug_mode_toggle() {
        set_debug_mode(true);
        assert!(is_debug_mode());
        
        set_debug_mode(false);
        assert!(!is_debug_mode());
    }

    // 注意：这个测试只能在 Windows 上运行
    #[test]
    #[cfg(windows)]
    fn test_short_path_no_spaces() {
        // 对于没有空格的路径，应该原样返回
        let path = "C:\\Windows\\System32";
        let result = get_short_path(path);
        // 只要不是 error 就可以，具体路径依赖系统
        assert!(result.is_ok());
        if let Ok(p) = result {
             assert_eq!(p, "C:\\Windows\\System32"); // 因为没空格
        }
    }

    #[test]
    fn test_compute_absolute_path() {
        let p = Path::new("test/../src/main.rs");
        // 假设当前目录下运行
        let abs = compute_absolute_path(p).unwrap();
        assert!(abs.is_absolute());
        // 验证逻辑消除是否生效 (字符串中不应包含 ..)
        let s = abs.to_string_lossy();
        assert!(!s.contains(".."));
    }
}
