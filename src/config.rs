use std::fmt;

use crate::cb_config::CbCompilerConfig;
use crate::debug_println;

/// 工具链解析失败错误类型
#[derive(Debug)]
pub enum ToolchainResolveError {
    /// CBP 引用了不存在于 default.conf 中的编译器 ID
    UnknownCompiler {
        compiler_id: String,
        available: Vec<String>,
    },
}

impl fmt::Display for ToolchainResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolchainResolveError::UnknownCompiler {
                compiler_id,
                available,
            } => {
                write!(
                    f,
                    "Unknown compiler '{}'. Available: {}",
                    compiler_id,
                    available.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for ToolchainResolveError {}

/// Hardcoded 工具链版本信息 (default.conf 中不包含此信息)
struct HardcodedToolchainInfo {
    version_name: String,
    gcc_version: String,
}

/// 获取 hardcoded 的版本映射
fn get_hardcoded_defaults(id: &str) -> Option<HardcodedToolchainInfo> {
    match id {
        "riscv32" => Some(HardcodedToolchainInfo {
            version_name: "V1".to_string(),
            gcc_version: "6.1.0".to_string(),
        }),
        "riscv32-v2" => Some(HardcodedToolchainInfo {
            version_name: "V2".to_string(),
            gcc_version: "10.2.0".to_string(),
        }),
        "riscv32-v3" => Some(HardcodedToolchainInfo {
            version_name: "V3".to_string(),
            gcc_version: "14.2.0".to_string(),
        }),
        _ => None,
    }
}

/// 获取所有 hardcoded 的编译器 ID 列表
fn get_hardcoded_ids() -> Vec<String> {
    vec![
        "riscv32".to_string(),
        "riscv32-v2".to_string(),
        "riscv32-v3".to_string(),
    ]
}

/// 从 compiler_id 推导 version_name
/// 例如 "riscv32-v4" -> "V4"，"riscv32" -> "V1"
fn derive_version_name(compiler_id: &str) -> String {
    if let Some(dash_pos) = compiler_id.rfind('-') {
        let suffix = &compiler_id[dash_pos + 1..];
        // 如果后缀以 'v' 开头，提取版本号
        if let Some(stripped) = suffix.strip_prefix('v') {
            format!("V{}", stripped)
        } else {
            format!("V{}", suffix)
        }
    } else {
        "V1".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct ToolchainConfig {
    pub version_name: String,                // e.g., "V2"
    pub gcc_version: String,                 // e.g., "10.2.0"
    pub toolchain_base_path: Option<String>, // 自定义工具链基础路径
    pub cb_include_dirs: Vec<String>,        // 来自 default.conf 的额外 include 路径
}

impl ToolchainConfig {
    /// 根据编译器 ID 和可选的 Code::Blocks 配置解析工具链
    ///
    /// 优先使用 default.conf 中的配置，降级到 hardcoded 默认值
    /// 当编译器 ID 在所有来源中都找不到时返回错误
    pub fn resolve_toolchain(
        compiler_id: &str,
        cb_config: Option<&CbCompilerConfig>,
    ) -> Result<Self, ToolchainResolveError> {
        debug_println!(
            "[DEBUG config] Resolving toolchain for compiler ID: {} (cb_config={})",
            compiler_id,
            cb_config.is_some()
        );

        // 获取 hardcoded 版本信息
        let hardcoded = get_hardcoded_defaults(compiler_id);
        let version_name = hardcoded
            .as_ref()
            .map(|h| h.version_name.clone())
            .unwrap_or_else(|| derive_version_name(compiler_id));
        let gcc_version = hardcoded
            .as_ref()
            .map(|h| h.gcc_version.clone())
            .unwrap_or_else(|| "14.2.0".to_string());

        // 尝试从 default.conf 获取 master_path
        let (toolchain_base_path, cb_include_dirs) = if let Some(config) = cb_config {
            if let Some(entry) = config.compilers.get(compiler_id) {
                debug_println!(
                    "[DEBUG config] Found compiler '{}' in default.conf, master_path={:?}",
                    compiler_id,
                    entry.master_path
                );
                (entry.master_path.clone(), entry.include_dirs.clone())
            } else {
                // default.conf 中找不到该编译器
                debug_println!(
                    "[DEBUG config] Compiler '{}' not found in default.conf",
                    compiler_id
                );

                // 如果有 hardcoded 默认值，允许降级使用
                if hardcoded.is_some() {
                    debug_println!(
                        "[DEBUG config] Using hardcoded default path for '{}'",
                        compiler_id
                    );
                    (None, Vec::new())
                } else {
                    // 既不在 default.conf 中也没有 hardcoded 默认值 -> 报错
                    let mut available: Vec<String> = config.compilers.keys().cloned().collect();
                    available.sort();
                    return Err(ToolchainResolveError::UnknownCompiler {
                        compiler_id: compiler_id.to_string(),
                        available,
                    });
                }
            }
        } else {
            // 没有 default.conf，降级到 hardcoded
            if hardcoded.is_none() {
                let available = get_hardcoded_ids();
                return Err(ToolchainResolveError::UnknownCompiler {
                    compiler_id: compiler_id.to_string(),
                    available,
                });
            }
            (None, Vec::new())
        };

        let config = ToolchainConfig {
            version_name,
            gcc_version,
            toolchain_base_path,
            cb_include_dirs,
        };
        debug_println!("[DEBUG config] Resolved toolchain: {:?}", config);
        Ok(config)
    }

    /// 旧 API：仅使用 hardcoded 默认值
    /// 保留向后兼容 (parser.rs 等调用点)
    pub fn from_compiler_id(id: &str) -> Option<Self> {
        debug_println!(
            "[DEBUG config] Creating toolchain config for compiler ID: {}",
            id
        );
        let config = Self::resolve_toolchain(id, None).ok();
        debug_println!(
            "[DEBUG config] Toolchain config created: {:?}",
            config.is_some()
        );
        config
    }

    /// 获取工具链基础路径
    pub fn get_base_path(&self) -> String {
        debug_println!("[DEBUG config] Getting toolchain base path...");
        let path = if let Some(custom_path) = &self.toolchain_base_path {
            debug_println!(
                "[DEBUG config] Using custom toolchain path: {}",
                custom_path
            );
            custom_path.clone()
        } else {
            let default_path = format!(
                "C:\\Program Files (x86)\\RV32-Toolchain\\RV32-{}",
                self.version_name
            );
            debug_println!(
                "[DEBUG config] Using default toolchain path: {}",
                default_path
            );
            default_path
        };
        debug_println!("[DEBUG config] Final base path: {}", path);
        path
    }

    pub fn compiler_path(&self) -> String {
        debug_println!("[DEBUG config] Building compiler path...");
        let base_path = self.get_base_path();
        debug_println!("[DEBUG config] Base path: {}", base_path);
        let compiler_path = format!("{}\\bin\\riscv32-elf-gcc.exe", base_path);
        debug_println!("[DEBUG config] Final compiler path: {}", compiler_path);
        debug_println!(
            "[DEBUG config] Compiler path exists: {}",
            std::path::Path::new(&compiler_path).exists()
        );
        compiler_path
    }

    /// 获取链接器路径，根据类型返回gcc或ld
    pub fn linker_path(&self, linker_type: &str) -> String {
        debug_println!(
            "[DEBUG config] Building linker path for type: {}",
            linker_type
        );
        let base_path = self.get_base_path();
        debug_println!("[DEBUG config] Base path: {}", base_path);

        let linker_path = if linker_type == "ld" {
            format!("{}\\bin\\riscv32-elf-ld.exe", base_path)
        } else {
            // 默认使用gcc作为链接器
            self.compiler_path()
        };

        debug_println!("[DEBUG config] Final linker path: {}", linker_path);
        debug_println!(
            "[DEBUG config] Linker path exists: {}",
            std::path::Path::new(&linker_path).exists()
        );
        linker_path
    }

    /// 获取ar路径，用于创建静态库
    pub fn ar_path(&self) -> String {
        debug_println!("[DEBUG config] Building ar path...");
        let base_path = self.get_base_path();
        debug_println!("[DEBUG config] Base path: {}", base_path);
        let ar_path = format!("{}\\bin\\riscv32-elf-ar.exe", base_path);
        debug_println!("[DEBUG config] Final ar path: {}", ar_path);
        debug_println!(
            "[DEBUG config] Ar path exists: {}",
            std::path::Path::new(&ar_path).exists()
        );
        ar_path
    }

    pub fn include_paths(&self) -> Vec<String> {
        debug_println!("[DEBUG config] Building include paths...");
        let base = self.get_base_path();
        let gcc_ver = &self.gcc_version;
        debug_println!("[DEBUG config] Base path: {}", base);

        let mut paths = Vec::new();

        // 工具链标准 include 路径
        let path1 = format!("{}\\lib\\gcc\\riscv32-elf\\{}\\include", base, gcc_ver);
        let path2 = format!(
            "{}\\lib\\gcc\\riscv32-elf\\{}\\include-fixed",
            base, gcc_ver
        );
        let path3 = format!("{}\\riscv32-elf\\include", base);

        debug_println!("[DEBUG config] Include path 1: {}", path1);
        debug_println!(
            "[DEBUG config] Include path 1 exists: {}",
            std::path::Path::new(&path1).exists()
        );
        debug_println!("[DEBUG config] Include path 2: {}", path2);
        debug_println!(
            "[DEBUG config] Include path 2 exists: {}",
            std::path::Path::new(&path2).exists()
        );
        debug_println!("[DEBUG config] Include path 3: {}", path3);
        debug_println!(
            "[DEBUG config] Include path 3 exists: {}",
            std::path::Path::new(&path3).exists()
        );

        paths.push(path1);
        paths.push(path2);
        paths.push(path3);

        // 追加来自 default.conf 的额外 include 路径
        for dir in &self.cb_include_dirs {
            debug_println!("[DEBUG config] CB include dir: {}", dir);
            paths.push(format!("-I{}", dir));
        }

        paths
    }

    /// 检查编译器路径是否存在
    pub fn is_compiler_available(&self) -> bool {
        let path = self.compiler_path();
        debug_println!(
            "[DEBUG config] Checking if compiler is available at: {}",
            path
        );
        let available = std::path::Path::new(&path).exists();
        debug_println!("[DEBUG config] Compiler available: {}", available);
        available
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cb_config::CbCompilerEntry;
    use std::collections::HashMap;

    #[test]
    fn test_toolchain_version_mapping() {
        let v2 = ToolchainConfig::from_compiler_id("riscv32-v2").unwrap();
        assert_eq!(v2.version_name, "V2");
        assert_eq!(v2.gcc_version, "10.2.0");

        let v3 = ToolchainConfig::from_compiler_id("riscv32-v3").unwrap();
        assert_eq!(v3.version_name, "V3");
    }

    #[test]
    fn test_invalid_compiler_id() {
        let invalid = ToolchainConfig::from_compiler_id("unknown-compiler");
        assert!(invalid.is_none());
    }

    #[test]
    fn test_path_generation() {
        let config = ToolchainConfig {
            version_name: "TestVer".to_string(),
            gcc_version: "1.0.0".to_string(),
            toolchain_base_path: Some("C:\\CustomToolchain".to_string()),
            cb_include_dirs: Vec::new(),
        };

        // 测试自定义路径
        assert_eq!(config.compiler_path(), "C:\\CustomToolchain\\bin\\riscv32-elf-gcc.exe");
        assert_eq!(config.ar_path(), "C:\\CustomToolchain\\bin\\riscv32-elf-ar.exe");

        // 测试 Linker 逻辑 (gcc vs ld)
        assert!(config.linker_path("gcc").ends_with("gcc.exe"));
        assert!(config.linker_path("ld").ends_with("ld.exe"));
    }

    #[test]
    fn test_resolve_toolchain_from_cb_config() {
        let mut compilers = HashMap::new();
        compilers.insert(
            "riscv32-v2".to_string(),
            CbCompilerEntry {
                compiler_id: "riscv32-v2".to_string(),
                master_path: Some("D:\\CustomToolchain".to_string()),
                include_dirs: vec!["D:\\extra\\include".to_string()],
                library_dirs: Vec::new(),
            },
        );
        let cb_config = CbCompilerConfig {
            compilers,
            default_compiler: None,
        };

        let toolchain =
            ToolchainConfig::resolve_toolchain("riscv32-v2", Some(&cb_config)).unwrap();
        assert_eq!(toolchain.toolchain_base_path, Some("D:\\CustomToolchain".to_string()));
        assert_eq!(toolchain.cb_include_dirs, vec!["D:\\extra\\include"]);
        assert_eq!(toolchain.compiler_path(), "D:\\CustomToolchain\\bin\\riscv32-elf-gcc.exe");
    }

    #[test]
    fn test_resolve_toolchain_unknown_with_cb_config() {
        let mut compilers = HashMap::new();
        compilers.insert(
            "riscv32-v2".to_string(),
            CbCompilerEntry {
                compiler_id: "riscv32-v2".to_string(),
                master_path: Some("D:\\V2".to_string()),
                include_dirs: Vec::new(),
                library_dirs: Vec::new(),
            },
        );
        let cb_config = CbCompilerConfig {
            compilers,
            default_compiler: None,
        };

        let result = ToolchainConfig::resolve_toolchain("unknown-compiler", Some(&cb_config));
        assert!(result.is_err());
        if let Err(ToolchainResolveError::UnknownCompiler { compiler_id, available }) = result {
            assert_eq!(compiler_id, "unknown-compiler");
            assert!(available.contains(&"riscv32-v2".to_string()));
        } else {
            panic!("Expected UnknownCompiler error");
        }
    }

    #[test]
    fn test_resolve_toolchain_fallback_when_no_cb_config() {
        // 没有 cb_config 时应使用 hardcoded 默认值
        let toolchain = ToolchainConfig::resolve_toolchain("riscv32-v2", None).unwrap();
        assert_eq!(toolchain.version_name, "V2");
        assert_eq!(toolchain.gcc_version, "10.2.0");
        assert!(toolchain.toolchain_base_path.is_none());
    }

    #[test]
    fn test_resolve_toolchain_hardcoded_id_not_in_cb_config() {
        // hardcoded ID 不在 cb_config 中，但有 hardcoded 默认值 -> 降级成功
        let cb_config = CbCompilerConfig {
            compilers: HashMap::new(),
            default_compiler: None,
        };

        let toolchain =
            ToolchainConfig::resolve_toolchain("riscv32-v2", Some(&cb_config)).unwrap();
        assert_eq!(toolchain.version_name, "V2");
        assert!(toolchain.toolchain_base_path.is_none()); // 降级到 hardcoded 默认路径
    }

    #[test]
    fn test_cb_include_dirs_appended() {
        let config = ToolchainConfig {
            version_name: "TestVer".to_string(),
            gcc_version: "1.0.0".to_string(),
            toolchain_base_path: Some("C:\\CustomToolchain".to_string()),
            cb_include_dirs: vec!["D:\\extra1".to_string(), "D:\\extra2".to_string()],
        };

        let paths = config.include_paths();
        // 标准路径 + CB 额外路径
        assert!(paths.len() >= 5);
        assert!(paths.iter().any(|p| p == "-ID:\\extra1"));
        assert!(paths.iter().any(|p| p == "-ID:\\extra2"));
    }

    #[test]
    fn test_derive_version_name() {
        assert_eq!(derive_version_name("riscv32-v4"), "V4");
        assert_eq!(derive_version_name("riscv32-v10"), "V10");
        assert_eq!(derive_version_name("riscv32"), "V1");
        assert_eq!(derive_version_name("riscv32-custom"), "Vcustom");
    }
}
