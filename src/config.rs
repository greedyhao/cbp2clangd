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

#[derive(Debug, Clone)]
pub struct ToolchainConfig {
    /// 工具链基础路径（来自 default.conf 的 MASTER_PATH）
    pub toolchain_base_path: String,
    /// C 编译器可执行文件名（来自 default.conf 的 C_COMPILER），如 "riscv32-elf-gcc.exe"
    pub c_compiler: Option<String>,
    /// C++ 编译器可执行文件名（来自 default.conf 的 CPP_COMPILER）
    pub cpp_compiler: Option<String>,
    /// 链接器可执行文件名（来自 default.conf 的 LINKER）
    pub linker: Option<String>,
    /// 库管理器可执行文件名（来自 default.conf 的 LIB_LINKER）
    pub lib_linker: Option<String>,
    /// 来自 default.conf 的额外 include 路径
    pub cb_include_dirs: Vec<String>,
}

impl ToolchainConfig {
    /// 根据编译器 ID 和 Code::Blocks 配置解析工具链
    ///
    /// 从 default.conf 中查找编译器 ID，提取 MASTER_PATH 和编译工具名。
    /// 查找顺序：
    ///   1. 精确匹配 XML 标签名（如 `riscv32_v2`）
    ///   2. 不区分大小写匹配 `<NAME>` 字段（如 `RISCV32-V2` 匹配条目 `riscv32_v2`）
    /// 如果找不到对应的编译器，返回错误。
    pub fn resolve_toolchain(
        compiler_id: &str,
        cb_config: &CbCompilerConfig,
    ) -> Result<Self, ToolchainResolveError> {
        debug_println!(
            "[DEBUG config] Resolving toolchain for compiler ID: {}",
            compiler_id,
        );

        // 1. 先尝试精确匹配 XML 标签名
        let entry = cb_config.compilers.get(compiler_id);
        // 2. 未命中则遍历所有条目，不区分大小写匹配 NAME 字段
        let entry = entry.or_else(|| {
            let compiler_lower = compiler_id.to_lowercase();
            cb_config.compilers.values().find(|e| {
                e.name.as_deref().map(|n| n.to_lowercase() == compiler_lower).unwrap_or(false)
            })
        });

        let entry = entry.ok_or_else(|| {
            let mut available: Vec<String> = cb_config.compilers.keys().cloned().collect();
            available.sort();
            ToolchainResolveError::UnknownCompiler {
                compiler_id: compiler_id.to_string(),
                available,
            }
        })?;

        let toolchain_base_path = entry.master_path.clone().ok_or_else(|| {
            // default.conf 中有该条目但没有 MASTER_PATH —— 视为配置不完整
            let mut available: Vec<String> = cb_config.compilers.keys().cloned().collect();
            available.sort();
            ToolchainResolveError::UnknownCompiler {
                compiler_id: compiler_id.to_string(),
                available,
            }
        })?;

        let config = ToolchainConfig {
            toolchain_base_path,
            c_compiler: entry.c_compiler.clone(),
            cpp_compiler: entry.cpp_compiler.clone(),
            linker: entry.linker.clone(),
            lib_linker: entry.lib_linker.clone(),
            cb_include_dirs: entry.include_dirs.clone(),
        };

        debug_println!("[DEBUG config] Resolved toolchain: {:?}", config);
        Ok(config)
    }

    /// 获取工具链基础路径
    pub fn get_base_path(&self) -> &str {
        &self.toolchain_base_path
    }

    /// C 编译器完整路径
    pub fn compiler_path(&self) -> String {
        let exe = self.c_compiler.as_deref().unwrap_or("gcc.exe");
        format!("{}\\bin\\{}", self.toolchain_base_path, exe)
    }

    /// C++ 编译器完整路径
    pub fn cpp_compiler_path(&self) -> String {
        let exe = self.cpp_compiler.as_deref().unwrap_or("g++.exe");
        format!("{}\\bin\\{}", self.toolchain_base_path, exe)
    }

    /// 获取链接器路径，根据类型返回 gcc 或 ld
    pub fn linker_path(&self, linker_type: &str) -> String {
        if linker_type == "ld" {
            let exe = self.linker.as_deref().unwrap_or("ld.exe");
            format!("{}\\bin\\{}", self.toolchain_base_path, exe)
        } else {
            // 默认使用 C 编译器作为链接器
            self.compiler_path()
        }
    }

    /// 获取 ar 路径，用于创建静态库
    pub fn ar_path(&self) -> String {
        let exe = self.lib_linker.as_deref().unwrap_or("ar.exe");
        format!("{}\\bin\\{}", self.toolchain_base_path, exe)
    }

    /// 返回来自 default.conf 的额外 include 路径（不含 -I 前缀，调用方自行添加）
    pub fn include_paths(&self) -> &[String] {
        &self.cb_include_dirs
    }

    /// 检查编译器是否可用（可执行文件是否存在）
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

    fn make_cb_config(entries: Vec<(&str, &str, Option<&str>, Option<&str>, Option<&str>, Option<&str>)>) -> CbCompilerConfig {
        let mut compilers = HashMap::new();
        for (id, master_path, c_compiler, cpp_compiler, linker, lib_linker) in entries {
            compilers.insert(
                id.to_string(),
                CbCompilerEntry {
                    compiler_id: id.to_string(),
                    name: None,
                    master_path: Some(master_path.to_string()),
                    c_compiler: c_compiler.map(|s| s.to_string()),
                    cpp_compiler: cpp_compiler.map(|s| s.to_string()),
                    linker: linker.map(|s| s.to_string()),
                    lib_linker: lib_linker.map(|s| s.to_string()),
                    include_dirs: Vec::new(),
                    library_dirs: Vec::new(),
                },
            );
        }
        CbCompilerConfig {
            compilers,
            default_compiler: None,
        }
    }

    #[test]
    fn test_resolve_toolchain_from_cb_config() {
        let cb_config = make_cb_config(vec![
            ("riscv32-v2", "D:\\CustomToolchain", Some("riscv32-elf-gcc.exe"), None, None, None),
        ]);

        let toolchain = ToolchainConfig::resolve_toolchain("riscv32-v2", &cb_config).unwrap();
        assert_eq!(toolchain.toolchain_base_path, "D:\\CustomToolchain");
        assert_eq!(
            toolchain.compiler_path(),
            "D:\\CustomToolchain\\bin\\riscv32-elf-gcc.exe"
        );
        assert_eq!(
            toolchain.cpp_compiler_path(),
            "D:\\CustomToolchain\\bin\\g++.exe"
        ); // 默认
        assert_eq!(toolchain.linker_path("gcc"), toolchain.compiler_path());
        assert_eq!(
            toolchain.linker_path("ld"),
            "D:\\CustomToolchain\\bin\\ld.exe"
        ); // 默认 ld
        assert_eq!(
            toolchain.ar_path(),
            "D:\\CustomToolchain\\bin\\ar.exe"
        ); // 默认 ar
    }

    #[test]
    fn test_resolve_toolchain_with_all_fields() {
        let cb_config = make_cb_config(vec![
            ("mygcc", "C:\\MyToolchain",
                Some("gcc.exe"), Some("g++.exe"), Some("ld.exe"), Some("ar.exe")),
        ]);

        let toolchain = ToolchainConfig::resolve_toolchain("mygcc", &cb_config).unwrap();
        assert_eq!(toolchain.compiler_path(), "C:\\MyToolchain\\bin\\gcc.exe");
        assert_eq!(toolchain.cpp_compiler_path(), "C:\\MyToolchain\\bin\\g++.exe");
        assert_eq!(toolchain.linker_path("ld"), "C:\\MyToolchain\\bin\\ld.exe");
        assert_eq!(toolchain.ar_path(), "C:\\MyToolchain\\bin\\ar.exe");
        assert_eq!(toolchain.get_base_path(), "C:\\MyToolchain");
    }

    #[test]
    fn test_resolve_toolchain_unknown_compiler() {
        let cb_config = make_cb_config(vec![
            ("riscv32-v2", "D:\\V2", None, None, None, None),
        ]);

        let result = ToolchainConfig::resolve_toolchain("unknown-compiler", &cb_config);
        assert!(result.is_err());
        if let Err(ToolchainResolveError::UnknownCompiler { compiler_id, available }) = result {
            assert_eq!(compiler_id, "unknown-compiler");
            assert!(available.contains(&"riscv32-v2".to_string()));
        } else {
            panic!("Expected UnknownCompiler error");
        }
    }

    #[test]
    fn test_resolve_toolchain_missing_master_path() {
        let mut compilers = HashMap::new();
        compilers.insert(
            "broken".to_string(),
            CbCompilerEntry {
                compiler_id: "broken".to_string(),
                name: None,
                master_path: None,
                c_compiler: None,
                cpp_compiler: None,
                linker: None,
                lib_linker: None,
                include_dirs: Vec::new(),
                library_dirs: Vec::new(),
            },
        );
        let cb_config = CbCompilerConfig {
            compilers,
            default_compiler: None,
        };

        let result = ToolchainConfig::resolve_toolchain("broken", &cb_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_compiler_fallback_names() {
        let cb_config = make_cb_config(vec![
            ("gcc", "C:\\MinGW", None, None, None, None),
        ]);

        let toolchain = ToolchainConfig::resolve_toolchain("gcc", &cb_config).unwrap();
        // C_COMPILER 未设置 → 默认 gcc.exe
        assert_eq!(toolchain.compiler_path(), "C:\\MinGW\\bin\\gcc.exe");
        // LIB_LINKER 未设置 → 默认 ar.exe
        assert_eq!(toolchain.ar_path(), "C:\\MinGW\\bin\\ar.exe");
        // LINKER 未设置且 linker_type=ld → 默认 ld.exe
        assert_eq!(toolchain.linker_path("ld"), "C:\\MinGW\\bin\\ld.exe");
        // linker_type=gcc → 使用 compiler_path
        assert_eq!(toolchain.linker_path("gcc"), toolchain.compiler_path());
    }

    #[test]
    fn test_compiler_availability() {
        let cb_config = make_cb_config(vec![
            ("exists", "C:\\DoesNotExist", Some("nonexistent.exe"), None, None, None),
        ]);

        let toolchain = ToolchainConfig::resolve_toolchain("exists", &cb_config).unwrap();
        // 路径肯定不存在
        assert!(!toolchain.is_compiler_available());
    }

    #[test]
    fn test_include_paths() {
        let mut compilers = HashMap::new();
        compilers.insert(
            "test".to_string(),
            CbCompilerEntry {
                compiler_id: "test".to_string(),
                name: None,
                master_path: Some("C:\\Toolchain".to_string()),
                c_compiler: None,
                cpp_compiler: None,
                linker: None,
                lib_linker: None,
                include_dirs: vec!["C:\\inc1".to_string(), "C:\\inc2".to_string()],
                library_dirs: Vec::new(),
            },
        );
        let cb_config = CbCompilerConfig {
            compilers,
            default_compiler: None,
        };

        let toolchain = ToolchainConfig::resolve_toolchain("test", &cb_config).unwrap();
        let paths = toolchain.include_paths();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"C:\\inc1".to_string()));
        assert!(paths.contains(&"C:\\inc2".to_string()));
    }

    #[test]
    fn test_resolve_by_name_case_insensitive() {
        // XML 标签是 riscv32_v2，但 NAME 是 RISCV32-V2
        // CBP 文件中 <Option compiler="RISCV32-V2"/> 应该匹配
        let mut compilers = HashMap::new();
        compilers.insert(
            "riscv32_v2".to_string(),
            CbCompilerEntry {
                compiler_id: "riscv32_v2".to_string(),
                name: Some("RISCV32-V2".to_string()),
                master_path: Some("C:\\RV32-V2".to_string()),
                c_compiler: None,
                cpp_compiler: None,
                linker: None,
                lib_linker: None,
                include_dirs: Vec::new(),
                library_dirs: Vec::new(),
            },
        );
        let cb_config = CbCompilerConfig {
            compilers,
            default_compiler: None,
        };

        // 应该通过 NAME 不区分大小写匹配到 riscv32_v2
        let toolchain = ToolchainConfig::resolve_toolchain("RISCV32-V2", &cb_config).unwrap();
        assert_eq!(toolchain.toolchain_base_path, "C:\\RV32-V2");

        // 小写也应该匹配
        let toolchain = ToolchainConfig::resolve_toolchain("riscv32-v2", &cb_config).unwrap();
        assert_eq!(toolchain.toolchain_base_path, "C:\\RV32-V2");

        // 大小写混写也应该匹配
        let toolchain = ToolchainConfig::resolve_toolchain("Riscv32-V2", &cb_config).unwrap();
        assert_eq!(toolchain.toolchain_base_path, "C:\\RV32-V2");
    }

    #[test]
    fn test_resolve_by_tag_name_still_works() {
        // XML 标签名直接匹配仍然优先
        let mut compilers = HashMap::new();
        compilers.insert(
            "riscv32".to_string(),
            CbCompilerEntry {
                compiler_id: "riscv32".to_string(),
                name: None,  // 没有 NAME
                master_path: Some("C:\\RV32".to_string()),
                c_compiler: None,
                cpp_compiler: None,
                linker: None,
                lib_linker: None,
                include_dirs: Vec::new(),
                library_dirs: Vec::new(),
            },
        );
        let cb_config = CbCompilerConfig {
            compilers,
            default_compiler: None,
        };

        let toolchain = ToolchainConfig::resolve_toolchain("riscv32", &cb_config).unwrap();
        assert_eq!(toolchain.toolchain_base_path, "C:\\RV32");
    }
}
