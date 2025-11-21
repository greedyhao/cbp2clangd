use crate::models::MarchInfo;
use roxmltree::Document;
use std::collections::HashSet;

/// 项目信息结构
pub struct ProjectInfo {
    pub compiler_id: String,
    pub global_cflags: Vec<String>,
    pub include_dirs: Vec<String>,
    pub source_files: Vec<String>,
    pub march_info: MarchInfo,
}

/// 解析Code::Blocks项目文件
pub fn parse_cbp_file(xml_content: &str) -> Result<ProjectInfo, Box<dyn std::error::Error>> {
    let doc = Document::parse(xml_content)?;
    let root = doc.root_element();

    // === FileVersion 检查（保留）===
    if let Some(fv) = root
        .children()
        .find(|n| n.tag_name().name() == "FileVersion")
    {
        let major = fv.attribute("major").unwrap_or("?");
        let minor = fv.attribute("minor").unwrap_or("?");
        println!("FileVersion: {}.{}", major, minor);
        if let (Ok(maj), Ok(min)) = (major.parse::<u32>(), minor.parse::<u32>()) {
            if !(maj == 1 && min >= 6) {
                eprintln!("Warning: FileVersion may be incompatible.");
            }
        } else {
            eprintln!("Warning: Invalid FileVersion format.");
        }
    } else {
        eprintln!("Warning: No FileVersion found.");
    }

    let project = root
        .children()
        .find(|n| n.tag_name().name() == "Project")
        .ok_or("No <Project> found")?;

    // === 提取 compiler ID ===
    let mut compiler_id = "riscv32-v2".to_string(); // default
    for option in project
        .children()
        .filter(|n| n.tag_name().name() == "Option")
    {
        if let Some(comp) = option.attribute("compiler") {
            compiler_id = comp.to_string();
            break;
        }
    }
    println!("Detected compiler: {}", compiler_id);

    // === 全局编译选项 ===
    let mut global_cflags = Vec::new();
    let mut include_dirs = Vec::new();
    let mut march_info = MarchInfo::default();

    if let Some(compiler_node) = project
        .children()
        .find(|n| n.tag_name().name() == "Compiler")
    {
        for add in compiler_node
            .children()
            .filter(|n| n.tag_name().name() == "Add")
        {
            if let Some(opt) = add.attribute("option") {
                let opt_str = opt.to_string();
                global_cflags.push(opt_str.clone());

                // 检测并解析-march=指令
                if opt_str.starts_with("-march=") {
                    let march_value = opt_str.trim_start_matches("-march=");
                    march_info.full_march = opt_str.clone();

                    // 尝试分离基础部分和自定义扩展
                    // 标准RISC-V扩展通常是a, c, d, e, f, g, h, i, m, p, v等单个字母
                    // 自定义扩展通常以x开头，后面跟着更多字符
                    if let Some(x_index) = march_value.find('x') {
                        let base_part = &march_value[0..x_index];
                        if !base_part.is_empty() {
                            march_info.base_march = Some(format!("-march={}", base_part));
                            march_info.has_custom_extension = true;
                        }
                    }
                }
            }
            if let Some(dir) = add.attribute("directory") {
                include_dirs.push(format!("-I{}", dir));
            }
        }
    }

    // === 源文件 ===
    let mut source_files = Vec::new();
    let valid_exts: HashSet<&str> = ["c", "cpp", "C", "CPP"].iter().cloned().collect();

    for unit in project.children().filter(|n| n.tag_name().name() == "Unit") {
        if let Some(filename) = unit.attribute("filename") {
            let path = std::path::Path::new(filename);
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if valid_exts.contains(ext) {
                    source_files.push(filename.to_string());
                }
            }
        }
    }

    if source_files.is_empty() {
        return Err("No source files (.c/.cpp) found in project.".into());
    }

    Ok(ProjectInfo {
        compiler_id,
        global_cflags,
        include_dirs,
        source_files,
        march_info,
    })
}
