use serde::Serialize;

/// 编译命令结构，用于生成compile_commands.json
#[derive(Serialize)]
pub struct CompileCommand {
    pub directory: String,
    pub command: String,
    pub file: String,
}

/// RISC-V架构特性信息
#[derive(Debug, Default)]
pub struct MarchInfo {
    pub full_march: String,         // 完整的-march参数值
    pub base_march: Option<String>, // 基础部分（不带自定义扩展）
    pub has_custom_extension: bool, // 是否包含自定义扩展
}
