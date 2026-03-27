use serde::{Serialize, Deserialize};

/// 普通源文件信息，包含编译和链接标志
#[derive(Debug, Default, PartialEq)]
pub struct SourceFileInfo {
    pub filename: String,    // 文件名
    pub compile: bool,       // 是否编译
    pub link: bool,          // 是否链接
}

/// 编译命令结构，用于生成compile_commands.json
#[derive(Serialize, Deserialize)]
pub struct CompileCommand {
    pub directory: String,
    pub command: String,
    pub file: String,
}

/// 特殊文件构建信息
#[derive(Debug, Default, PartialEq)]
pub struct SpecialFileBuildInfo {
    pub filename: String, // 文件名
    #[allow(dead_code)]
    pub compiler_id: String, // 编译器ID
    pub build_command: String, // 构建命令模板
    pub compile: bool,      // 是否编译
    pub link: bool,         // 是否链接
}

/// RISC-V架构特性信息
#[derive(Debug, Default)]
pub struct MarchInfo {
    pub full_march: String,         // 完整的-march参数值
    pub base_march: Option<String>, // 基础部分（不带自定义扩展）
    pub has_custom_extension: bool, // 是否包含自定义扩展
}

/// 单个Build Target的配置信息
#[derive(Debug, Default)]
pub struct BuildTarget {
    pub name: String,                   // Target名称 (如 "Debug", "Release")
    pub output: String,                 // 输出文件路径
    pub object_output: String,          // 对象文件输出目录
    pub cflags: Vec<String>,            // 编译选项
    pub defines: Vec<String>,           // 宏定义 (-D)
    pub include_dirs: Vec<String>,      // 头文件目录 (-I)
    pub linker_options: Vec<String>,    // 链接器选项
    pub linker_libs: Vec<String>,       // 链接库
    pub linker_lib_dirs: Vec<String>,   // 库搜索路径
    pub march_info: MarchInfo,          // 架构信息
}
