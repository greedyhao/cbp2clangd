use crate::utils::debug_println;

#[derive(Debug, Clone)]
pub struct ToolchainConfig {
    pub version_name: String,                // e.g., "V2"
    pub gcc_version: String,                 // e.g., "10.2.0"
    pub toolchain_base_path: Option<String>, // 自定义工具链基础路径
}

impl ToolchainConfig {
    pub fn from_compiler_id(id: &str) -> Option<Self> {
        debug_println!(
            "[DEBUG config] Creating toolchain config for compiler ID: {}",
            id
        );
        let config = match id {
            "riscv32" => Some(ToolchainConfig {
                version_name: "V1".to_string(),
                gcc_version: "6.1.0".to_string(),
                toolchain_base_path: None, // 使用默认路径
            }),
            "riscv32-v2" => Some(ToolchainConfig {
                version_name: "V2".to_string(),
                gcc_version: "10.2.0".to_string(),
                toolchain_base_path: None, // 使用默认路径
            }),
            "riscv32-v3" => Some(ToolchainConfig {
                version_name: "V3".to_string(),
                gcc_version: "14.2.0".to_string(),
                toolchain_base_path: None, // 使用默认路径
            }),
            _ => {
                debug_println!("[DEBUG config] Unknown compiler ID: {}", id);
                None
            }
        };
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

        vec![path1, path2, path3]
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
