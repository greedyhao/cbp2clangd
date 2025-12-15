use crate::models::MarchInfo;
use roxmltree::Document;
use std::collections::HashSet;
use std::path::Path;

/// 项目信息结构
pub struct ProjectInfo {
    pub compiler_id: String,
    pub global_cflags: Vec<String>,
    pub include_dirs: Vec<String>,
    pub source_files: Vec<String>,
    pub march_info: MarchInfo,
    pub object_output: String,
    pub linker_options: Vec<String>,
    pub linker_libs: Vec<String>,
    pub linker_lib_dirs: Vec<String>,
    pub linker_type: String,
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
    let mut linker_options = Vec::new();
    let mut linker_libs = Vec::new();
    let mut linker_lib_dirs = Vec::new();

    // 解析Compiler节点
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
    
    // 解析Linker节点
    if let Some(linker_node) = project
        .children()
        .find(|n| n.tag_name().name() == "Linker")
    {
        for add in linker_node
            .children()
            .filter(|n| n.tag_name().name() == "Add")
        {
            if let Some(opt) = add.attribute("option") {
                linker_options.push(opt.to_string());
            }
            if let Some(lib) = add.attribute("library") {
                // 检查是否是路径（包含/或\）
                let lib_path = Path::new(lib);
                if lib_path.has_root() || lib.contains("/") || lib.contains("\\") {
                    // 带路径的库，直接使用完整路径
                    linker_libs.push(lib.to_string());
                } else {
                    // 不带路径的库，处理前缀
                    let processed_lib = if lib.starts_with("lib") {
                        // 去掉lib前缀，添加-l
                        format!("-l{}", &lib[3..])
                    } else {
                        // 直接添加-l
                        format!("-l{}", lib)
                    };
                    linker_libs.push(processed_lib);
                }
            }
            if let Some(dir) = add.attribute("directory") {
                linker_lib_dirs.push(format!("-L{}", dir));
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

    // === 解析object_output目录 ===
    let mut object_output = String::new();
    
    // 查找Build节点
    for build_node in project.children().filter(|n| n.tag_name().name() == "Build") {
        // 查找Target节点
        for target_node in build_node.children().filter(|n| n.tag_name().name() == "Target") {
            // 查找带有object_output属性的Option节点
            for option_node in target_node.children().filter(|n| n.tag_name().name() == "Option") {
                if let Some(obj_output) = option_node.attribute("object_output") {
                    object_output = obj_output.to_string();
                    break;
                }
            }
            // 找到一个就够了，跳出循环
            if !object_output.is_empty() {
                break;
            }
        }
        // 找到一个就够了，跳出循环
        if !object_output.is_empty() {
            break;
        }
    }
    
    // 如果没有找到object_output，使用默认值
    if object_output.is_empty() {
        object_output = "./".to_string();
    }

    Ok(ProjectInfo {
        compiler_id,
        global_cflags,
        include_dirs,
        source_files,
        march_info,
        object_output,
        linker_options,
        linker_libs,
        linker_lib_dirs,
        linker_type: "gcc".to_string(),
    })
}
