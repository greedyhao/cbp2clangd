use std::collections::HashMap;
use std::path::PathBuf;

use crate::debug_println;

/// Code::Blocks default.conf 中单个编译器条目
#[derive(Debug, Clone)]
pub struct CbCompilerEntry {
    pub compiler_id: String,
    /// 工具链安装根路径 (对应 MASTER_PATH)
    pub master_path: Option<String>,
    /// 额外的头文件目录 (对应 INCLUDE_DIRS，分号分隔)
    pub include_dirs: Vec<String>,
    /// 额外的库目录 (对应 LIBRARY_DIRS，分号分隔)
    pub library_dirs: Vec<String>,
}

/// 从 Code::Blocks default.conf 读取的编译器配置集合
#[derive(Debug, Clone)]
pub struct CbCompilerConfig {
    /// compiler_id -> CbCompilerEntry
    pub compilers: HashMap<String, CbCompilerEntry>,
    /// 默认编译器 ID
    pub default_compiler: Option<String>,
}

/// 定位 Code::Blocks default.conf 文件
/// 路径: %APPDATA%\CodeBlocks\default.conf
/// 文件不存在时返回 None
pub fn find_default_conf() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    let path = PathBuf::from(appdata).join("CodeBlocks").join("default.conf");
    debug_println!("[DEBUG cb_config] Looking for default.conf at: {}", path.display());
    if path.exists() {
        debug_println!("[DEBUG cb_config] Found default.conf");
        Some(path)
    } else {
        debug_println!("[DEBUG cb_config] default.conf not found");
        None
    }
}

/// 解析 Code::Blocks default.conf XML 内容
///
/// XML 结构示例:
/// ```xml
/// <CodeBlocksConfig version="1">
///   <compiler>
///     <DEFAULT_COMPILER><str><![CDATA[gcc]]></str></DEFAULT_COMPILER>
///     <sets>
///       <riscv32-v2>
///         <NAME><str><![CDATA[RISC-V 32-bit GCC V2]]></str></NAME>
///         <MASTER_PATH><str><![CDATA[C:\path\to\toolchain]]></str></MASTER_PATH>
///         <INCLUDE_DIRS><str><![CDATA[path1;path2;]]></str></INCLUDE_DIRS>
///       </riscv32-v2>
///     </sets>
///   </compiler>
/// </CodeBlocksConfig>
/// ```
pub fn parse_default_conf(xml_content: &str) -> Result<CbCompilerConfig, Box<dyn std::error::Error>> {
    let doc = roxmltree::Document::parse(xml_content)?;
    let root = doc.root_element();

    let mut compilers = HashMap::new();
    let mut default_compiler = None;

    // 查找 <compiler> 节点
    let compiler_node = root.children().find(|n| n.tag_name().name() == "compiler");

    if let Some(compiler_node) = compiler_node {
        // 提取默认编译器
        default_compiler = extract_str_field(&compiler_node, "DEFAULT_COMPILER");

        // 遍历 <sets> 下的编译器条目
        if let Some(sets_node) = compiler_node.children().find(|n| n.tag_name().name() == "sets") {
            for child in sets_node.children() {
                let tag = child.tag_name().name();
                // 跳过非编译器条目（如文本节点、set000 等）
                if tag.is_empty() || child.children().count() == 0 {
                    continue;
                }
                // 跳过 setNNN 格式的占位节点
                if tag.starts_with("set") && tag[3..].parse::<u32>().is_ok() {
                    continue;
                }

                let compiler_id = tag.to_string();
                let master_path = extract_str_field(&child, "MASTER_PATH");
                let include_dirs = extract_list_field(&child, "INCLUDE_DIRS");
                let library_dirs = extract_list_field(&child, "LIBRARY_DIRS");

                debug_println!(
                    "[DEBUG cb_config] Found compiler: id={}, master_path={:?}",
                    compiler_id,
                    master_path
                );

                compilers.insert(compiler_id.clone(), CbCompilerEntry {
                    compiler_id,
                    master_path,
                    include_dirs,
                    library_dirs,
                });
            }
        }
    }

    debug_println!(
        "[DEBUG cb_config] Parsed {} compiler entries, default={:?}",
        compilers.len(),
        default_compiler
    );

    Ok(CbCompilerConfig {
        compilers,
        default_compiler,
    })
}

/// 便捷函数：查找并加载 Code::Blocks 编译器配置
/// 文件不存在或读取失败时返回 None（静默降级到 hardcoded 默认值）
pub fn load_cb_compiler_config() -> Option<CbCompilerConfig> {
    let path = find_default_conf()?;
    let content = std::fs::read_to_string(&path).ok()?;
    parse_default_conf(&content).ok()
}

/// 从 XML 节点提取 `<TAG><str><![CDATA[value]]></str></TAG>` 格式的字符串值
fn extract_str_field(node: &roxmltree::Node, tag: &str) -> Option<String> {
    let field_node = node.children().find(|n| n.tag_name().name() == tag)?;
    let str_node = field_node.children().find(|n| n.tag_name().name() == "str")?;
    let text = str_node.text()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// 从 XML 节点提取分号分隔的列表字段
fn extract_list_field(node: &roxmltree::Node, tag: &str) -> Vec<String> {
    extract_str_field(node, tag)
        .map(|s| {
            s.split(';')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_default_conf() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocksConfig version="1">
    <compiler>
        <DEFAULT_COMPILER><str><![CDATA[gcc]]></str></DEFAULT_COMPILER>
        <sets>
            <riscv32-v2>
                <NAME><str><![CDATA[RISC-V 32-bit GCC V2]]></str></NAME>
                <MASTER_PATH><str><![CDATA[C:\Program Files (x86)\RV32-Toolchain\RV32-V2]]></str></MASTER_PATH>
            </riscv32-v2>
        </sets>
    </compiler>
</CodeBlocksConfig>"#;

        let config = parse_default_conf(xml).unwrap();
        assert_eq!(config.default_compiler, Some("gcc".to_string()));
        assert_eq!(config.compilers.len(), 1);

        let entry = config.compilers.get("riscv32-v2").unwrap();
        assert_eq!(
            entry.master_path,
            Some(r#"C:\Program Files (x86)\RV32-Toolchain\RV32-V2"#.to_string())
        );
    }

    #[test]
    fn test_parse_empty_compiler_section() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocksConfig version="1">
    <compiler>
        <sets />
    </compiler>
</CodeBlocksConfig>"#;

        let config = parse_default_conf(xml).unwrap();
        assert!(config.compilers.is_empty());
        assert!(config.default_compiler.is_none());
    }

    #[test]
    fn test_parse_missing_master_path() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocksConfig version="1">
    <compiler>
        <sets>
            <riscv32>
                <NAME><str><![CDATA[RISC-V 32-bit GCC]]></str></NAME>
            </riscv32>
        </sets>
    </compiler>
</CodeBlocksConfig>"#;

        let config = parse_default_conf(xml).unwrap();
        let entry = config.compilers.get("riscv32").unwrap();
        assert!(entry.master_path.is_none());
    }

    #[test]
    fn test_parse_include_dirs_semicolon_delimited() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocksConfig version="1">
    <compiler>
        <sets>
            <gcc>
                <MASTER_PATH><str><![CDATA[C:\MinGW]]></str></MASTER_PATH>
                <INCLUDE_DIRS><str><![CDATA[C:\include1;C:\include2;]]></str></INCLUDE_DIRS>
                <LIBRARY_DIRS><str><![CDATA[C:\lib1;C:\lib2;]]></str></LIBRARY_DIRS>
            </gcc>
        </sets>
    </compiler>
</CodeBlocksConfig>"#;

        let config = parse_default_conf(xml).unwrap();
        let entry = config.compilers.get("gcc").unwrap();
        assert_eq!(entry.include_dirs, vec!["C:\\include1", "C:\\include2"]);
        assert_eq!(entry.library_dirs, vec!["C:\\lib1", "C:\\lib2"]);
    }

    #[test]
    fn test_parse_multiple_compilers() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocksConfig version="1">
    <compiler>
        <DEFAULT_COMPILER><str><![CDATA[riscv32-v2]]></str></DEFAULT_COMPILER>
        <sets>
            <riscv32>
                <MASTER_PATH><str><![CDATA[C:\RV32-V1]]></str></MASTER_PATH>
            </riscv32>
            <riscv32-v2>
                <MASTER_PATH><str><![CDATA[C:\RV32-V2]]></str></MASTER_PATH>
            </riscv32-v2>
            <riscv32-v3>
                <MASTER_PATH><str><![CDATA[C:\RV32-V3]]></str></MASTER_PATH>
            </riscv32-v3>
        </sets>
    </compiler>
</CodeBlocksConfig>"#;

        let config = parse_default_conf(xml).unwrap();
        assert_eq!(config.compilers.len(), 3);
        assert_eq!(config.default_compiler, Some("riscv32-v2".to_string()));

        assert_eq!(
            config.compilers.get("riscv32").unwrap().master_path,
            Some("C:\\RV32-V1".to_string())
        );
        assert_eq!(
            config.compilers.get("riscv32-v2").unwrap().master_path,
            Some("C:\\RV32-V2".to_string())
        );
        assert_eq!(
            config.compilers.get("riscv32-v3").unwrap().master_path,
            Some("C:\\RV32-V3".to_string())
        );
    }

    #[test]
    fn test_parse_no_compiler_section() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocksConfig version="1">
    <other />
</CodeBlocksConfig>"#;

        let config = parse_default_conf(xml).unwrap();
        assert!(config.compilers.is_empty());
    }

    #[test]
    fn test_skip_set_nnn_nodes() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocksConfig version="1">
    <compiler>
        <sets>
            <riscv32-v2>
                <MASTER_PATH><str><![CDATA[C:\RV32-V2]]></str></MASTER_PATH>
            </riscv32-v2>
            <set000 />
            <set001 />
        </sets>
    </compiler>
</CodeBlocksConfig>"#;

        let config = parse_default_conf(xml).unwrap();
        // set000 和 set001 应该被跳过
        assert_eq!(config.compilers.len(), 1);
        assert!(config.compilers.contains_key("riscv32-v2"));
    }
}
