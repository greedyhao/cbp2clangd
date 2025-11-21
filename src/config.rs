#[derive(Debug, Clone)]
pub struct ToolchainConfig {
    pub version_name: String, // e.g., "V2"
    pub gcc_version: String,  // e.g., "10.2.0"
}

impl ToolchainConfig {
    pub fn from_compiler_id(id: &str) -> Option<Self> {
        match id {
            "riscv32-v1" => Some(ToolchainConfig {
                version_name: "V1".to_string(),
                gcc_version: "6.1.0".to_string(),
            }),
            "riscv32-v2" => Some(ToolchainConfig {
                version_name: "V2".to_string(),
                gcc_version: "10.2.0".to_string(),
            }),
            "riscv32-v3" => Some(ToolchainConfig {
                version_name: "V3".to_string(),
                gcc_version: "14.2.0".to_string(),
            }),
            _ => None,
        }
    }

    pub fn compiler_path(&self) -> String {
        format!(
            "C:\\Program Files (x86)\\RV32-Toolchain\\RV32-{}\\bin\\riscv32-elf-gcc.exe",
            self.version_name
        )
    }

    pub fn include_paths(&self) -> Vec<String> {
        let base = format!(
            "C:\\Program Files (x86)\\RV32-Toolchain\\RV32-{}",
            self.version_name
        );
        let gcc_ver = &self.gcc_version;
        vec![
            format!("{}\\lib\\gcc\\riscv32-elf\\{}\\include", base, gcc_ver),
            format!(
                "{}\\lib\\gcc\\riscv32-elf\\{}\\include-fixed",
                base, gcc_ver
            ),
            format!("{}\\riscv32-elf\\include", base),
        ]
    }
}
