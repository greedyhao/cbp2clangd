use serde::Deserialize;

/// YAML 文件中单个编译器配置项
#[derive(Debug, Deserialize)]
pub struct CompilerYamlEntry {
    pub name: String,
    pub master_path: String,
    pub c_compiler: Option<String>,
    pub cpp_compiler: Option<String>,
    pub linker: Option<String>,
    pub lib_linker: Option<String>,
    /// 父编译器 ID，默认 "gcc"
    #[serde(default = "default_parent")]
    pub parent: String,
}

fn default_parent() -> String {
    "gcc".to_string()
}

/// YAML 文件根结构
#[derive(Debug, Deserialize)]
pub struct CompilerYamlConfig {
    pub compilers: Vec<CompilerYamlEntry>,
}

/// 将 NAME 转换为 compiler_id：小写 + 连字符/空格 → 下划线
pub fn name_to_compiler_id(name: &str) -> String {
    name.to_lowercase().replace('-', "_").replace(' ', "_")
}

/// 生成编译器条目的 XML 片段（匹配 default.conf 缩进风格）
fn generate_entry_xml(entry: &CompilerYamlEntry, compiler_id: &str) -> String {
    let t3 = "\t\t\t"; // entry 标签
    let t4 = "\t\t\t\t"; // field 标签
    let t5 = "\t\t\t\t\t"; // <str>
    let t6 = "\t\t\t\t\t\t"; // CDATA

    let mut xml = String::new();
    xml.push_str(&format!("{}<{}>\n", t3, compiler_id));

    // 辅助宏：生成带 CDATA 的字段
    macro_rules! field {
        ($tag:expr, $value:expr) => {
            format!(
                "{}<{}>\n{}<str>\n{}{}\n{}</str>\n{}</{}>\n",
                t4,
                $tag,
                t5,
                t6,
                cdata($value),
                t5,
                t4,
                $tag
            )
        };
    }

    // NAME
    xml.push_str(&field!("NAME", &entry.name));

    // PARENT
    xml.push_str(&field!("PARENT", &entry.parent));

    // MASTER_PATH
    xml.push_str(&field!("MASTER_PATH", &entry.master_path));

    // C_COMPILER
    if let Some(ref cc) = entry.c_compiler {
        xml.push_str(&field!("C_COMPILER", cc));
    }

    // CPP_COMPILER
    if let Some(ref cpp) = entry.cpp_compiler {
        xml.push_str(&field!("CPP_COMPILER", cpp));
    }

    // LINKER
    if let Some(ref linker) = entry.linker {
        xml.push_str(&field!("LINKER", linker));
    }

    // LIB_LINKER
    if let Some(ref lib_linker) = entry.lib_linker {
        xml.push_str(&field!("LIB_LINKER", lib_linker));
    }

    xml.push_str(&format!("{}</{}>\n", t3, compiler_id));
    xml
}

fn cdata(value: &str) -> String {
    format!("<![CDATA[{}]]>", value)
}

/// 查找 compiler_id 在 <sets> 或 <user_sets> 中的位置
/// 返回 (start_pos, end_pos) 或 None
fn find_entry_in_content(content: &str, compiler_id: &str) -> Option<(usize, usize)> {
    let start_tag = format!("<{}>", compiler_id);
    let end_tag = format!("</{}>", compiler_id);

    // 在 <sets> 和 <user_sets> 范围内查找
    // 简单做法：全局搜索匹配的标签对
    if let Some(start) = content.find(&start_tag) {
        if let Some(end) = content[start..].find(&end_tag) {
            let end_pos = start + end + end_tag.len();
            return Some((start, end_pos));
        }
    }
    None
}

/// 确保 <user_sets> 标签存在，返回其关闭标签前的位置
fn ensure_user_sets(content: &mut String) -> Option<usize> {
    // 检查是否有 </user_sets>
    if let Some(pos) = content.find("</user_sets>") {
        return Some(pos);
    }

    // 检查是否有 <user_sets/> (自闭合)
    let empty_tag = "<user_sets/>";
    if let Some(pos) = content.find(empty_tag) {
        // 替换为开放-关闭对
        let replacement = "<user_sets>\n\t\t\t</user_sets>";
        content.replace_range(pos..pos + empty_tag.len(), replacement);
        // 返回关闭标签前的位置
        return content.find("</user_sets>");
    }

    // 都不存在：在 </sets> 前插入 <user_sets>
    let sets_close = "</sets>";
    if let Some(pos) = content.find(sets_close) {
        let insert = "\t\t<user_sets>\n\t\t</user_sets>\n\t\t";
        content.insert_str(pos, insert);
        // 重新查找关闭标签
        return content.find("</user_sets>");
    }

    None
}

/// 应用 YAML 配置到 default.conf 内容
/// 返回修改后的内容
pub fn apply_config_to_content(
    content: &str,
    config: &CompilerYamlConfig,
) -> Result<String, String> {
    let mut result = content.to_string();

    for entry in &config.compilers {
        let compiler_id = name_to_compiler_id(&entry.name);
        let entry_xml = generate_entry_xml(entry, &compiler_id);

        // 查找是否已存在
        if let Some((start, end)) = find_entry_in_content(&result, &compiler_id) {
            // 更新：替换旧条目
            result.replace_range(start..end, &entry_xml.trim_end());
            println!("  Updated compiler '{}' (id={})", entry.name, compiler_id);
        } else {
            // 新增：插入到 <user_sets>
            let insert_pos = ensure_user_sets(&mut result)
                .ok_or_else(|| "Cannot find <sets> section in default.conf".to_string())?;
            result.insert_str(insert_pos, &entry_xml);
            println!("  Added compiler '{}' (id={})", entry.name, compiler_id);
        }
    }

    Ok(result)
}

/// 从 YAML 文件路径加载配置并应用到 default.conf
pub fn apply_config_file(
    yaml_path: &std::path::Path,
    conf_path: &std::path::Path,
) -> Result<(), String> {
    // 读取 YAML
    let yaml_content = std::fs::read_to_string(yaml_path)
        .map_err(|e| format!("Failed to read YAML file '{}': {}", yaml_path.display(), e))?;
    let config: CompilerYamlConfig =
        serde_yaml::from_str(&yaml_content).map_err(|e| format!("Failed to parse YAML: {}", e))?;

    if config.compilers.is_empty() {
        return Err("No compilers defined in YAML file".to_string());
    }

    // 读取 default.conf
    let conf_content = std::fs::read_to_string(conf_path)
        .map_err(|e| format!("Failed to read '{}': {}", conf_path.display(), e))?;

    // 验证 XML 结构
    if let Err(e) = roxmltree::Document::parse(&conf_content) {
        return Err(format!("Invalid default.conf XML: {}", e));
    }

    println!(
        "Applying {} compiler configuration(s):",
        config.compilers.len()
    );

    // 应用修改
    let new_content = apply_config_to_content(&conf_content, &config)?;

    // 验证修改后的 XML
    if let Err(e) = roxmltree::Document::parse(&new_content) {
        return Err(format!("Generated XML is invalid: {}", e));
    }

    // 写回
    // 先备份
    let backup_path = conf_path.with_extension("conf.bak");
    std::fs::copy(conf_path, &backup_path)
        .map_err(|e| format!("Failed to create backup '{}': {}", backup_path.display(), e))?;
    println!("  Backup saved to: {}", backup_path.display());

    std::fs::write(conf_path, &new_content)
        .map_err(|e| format!("Failed to write '{}': {}", conf_path.display(), e))?;
    println!("  Updated: {}", conf_path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_to_compiler_id() {
        assert_eq!(name_to_compiler_id("RISCV32-V2"), "riscv32_v2");
        assert_eq!(name_to_compiler_id("My Compiler"), "my_compiler");
        assert_eq!(name_to_compiler_id("GCC"), "gcc");
        assert_eq!(name_to_compiler_id("ARM-GCC-10"), "arm_gcc_10");
    }

    #[test]
    fn test_generate_entry_xml_roundtrip() {
        let entry = CompilerYamlEntry {
            name: "RISCV32-V4".to_string(),
            master_path: "C:\\Toolchain\\V4".to_string(),
            c_compiler: Some("riscv32-elf-gcc.exe".to_string()),
            cpp_compiler: None,
            linker: None,
            lib_linker: None,
            parent: "gcc".to_string(),
        };
        let id = name_to_compiler_id(&entry.name);
        let xml = generate_entry_xml(&entry, &id);

        assert!(xml.contains("<riscv32_v4>"));
        assert!(xml.contains("</riscv32_v4>"));
        assert!(xml.contains("<![CDATA[RISCV32-V4]]>"));
        assert!(xml.contains("<![CDATA[C:\\Toolchain\\V4]]>"));
        assert!(xml.contains("<![CDATA[gcc]]>"));
        assert!(xml.contains("<![CDATA[riscv32-elf-gcc.exe]]>"));
        // 未设置的字段不应出现
        assert!(!xml.contains("CPP_COMPILER"));
        assert!(!xml.contains("LINKER"));
        assert!(!xml.contains("LIB_LINKER"));
    }

    #[test]
    fn test_find_entry_in_content() {
        let content = "<sets>\n\t\t\t<gcc>\n\t\t\t\t<NAME>...</NAME>\n\t\t\t</gcc>\n\t\t\t<clang>\n\t\t\t</clang>\n\t\t</sets>";
        assert!(find_entry_in_content(content, "gcc").is_some());
        assert!(find_entry_in_content(content, "clang").is_some());
        assert!(find_entry_in_content(content, "nonexistent").is_none());
    }

    #[test]
    fn test_apply_config_add_new_entry() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocksConfig version="1">
    <compiler>
        <sets>
            <gcc>
                <MASTER_PATH><str><![CDATA[C:\MinGW]]></str></MASTER_PATH>
            </gcc>
        </sets>
    </compiler>
</CodeBlocksConfig>"#;

        let config = CompilerYamlConfig {
            compilers: vec![CompilerYamlEntry {
                name: "MY-TOOLCHAIN".to_string(),
                master_path: "C:\\MyTools".to_string(),
                c_compiler: Some("my-gcc.exe".to_string()),
                cpp_compiler: None,
                linker: None,
                lib_linker: None,
                parent: "gcc".to_string(),
            }],
        };

        let result = apply_config_to_content(content, &config).unwrap();
        assert!(result.contains("<my_toolchain>"));
        assert!(result.contains("<![CDATA[MY-TOOLCHAIN]]>"));
        assert!(result.contains("<![CDATA[C:\\MyTools]]>"));
        assert!(result.contains("<![CDATA[my-gcc.exe]]>"));

        // 验证仍为合法 XML
        assert!(roxmltree::Document::parse(&result).is_ok());
    }

    #[test]
    fn test_apply_config_update_existing() {
        let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocksConfig version="1">
    <compiler>
        <sets>
            <old_name>
                <NAME><str><![CDATA[OLD-NAME]]></str></NAME>
                <MASTER_PATH><str><![CDATA[C:\Old]]></str></MASTER_PATH>
            </old_name>
        </sets>
        <user_sets>
        </user_sets>
    </compiler>
</CodeBlocksConfig>"#;

        let config = CompilerYamlConfig {
            compilers: vec![CompilerYamlEntry {
                name: "OLD-NAME".to_string(),
                master_path: "C:\\New".to_string(),
                c_compiler: Some("new-gcc.exe".to_string()),
                cpp_compiler: None,
                linker: None,
                lib_linker: None,
                parent: "gcc".to_string(),
            }],
        };

        let result = apply_config_to_content(content, &config).unwrap();
        // 旧路径不应该出现了
        assert!(!result.contains("C:\\Old"));
        // 新路径应该出现
        assert!(result.contains("<![CDATA[C:\\New]]>"));
        assert!(result.contains("<![CDATA[new-gcc.exe]]>"));

        assert!(roxmltree::Document::parse(&result).is_ok());
    }

    #[test]
    fn test_apply_config_default_parent() {
        let entry: CompilerYamlEntry = serde_yaml::from_str(
            r#"
name: "TEST"
master_path: "C:\\test"
"#,
        )
        .unwrap();
        assert_eq!(entry.parent, "gcc");
    }
}
