use crate::ToolchainConfig;
use crate::models::{MarchInfo, SpecialFileBuildInfo};
use roxmltree::Document;
use std::collections::HashSet;
use std::path::Path;

/// 项目信息结构
pub struct ProjectInfo {
    pub compiler_id: String,
    pub project_name: String,
    pub global_cflags: Vec<String>,
    pub include_dirs: Vec<String>,
    pub source_files: Vec<String>,
    pub special_files: Vec<SpecialFileBuildInfo>,
    pub prebuild_commands: Vec<String>,
    pub postbuild_commands: Vec<String>,
    pub march_info: MarchInfo,
    pub object_output: String,
    pub output: String,
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

    // === 提取项目名称 ===
    let mut project_name = "output".to_string(); // default
    for option in project
        .children()
        .filter(|n| n.tag_name().name() == "Option")
    {
        if let Some(title) = option.attribute("title") {
            project_name = title.to_string();
            break;
        }
    }
    println!("Project name: {}", project_name);

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
    let mut prebuild_commands = Vec::new();
    let mut postbuild_commands = Vec::new();

    // 用于存储Build/Target/Linker中的库，按顺序保存
    let mut build_target_linker_libs = Vec::new();
    // 用于快速检查Build/Target/Linker中的库，避免Project/Linker添加重复的库
    let mut build_target_lib_set = HashSet::new();

    // 解析Build/Target节点，获取库信息和宏定义
    for build_node in project
        .children()
        .filter(|n| n.tag_name().name() == "Build")
    {
        for target_node in build_node
            .children()
            .filter(|n| n.tag_name().name() == "Target")
        {
            // 处理Target下的Linker节点，获取库信息和库目录
            if let Some(linker_node) = target_node
                .children()
                .find(|n| n.tag_name().name() == "Linker")
            {
                for add in linker_node
                    .children()
                    .filter(|n| n.tag_name().name() == "Add")
                {
                    if let Some(lib) = add.attribute("library") {
                        // 检查是否是路径（包含/或\）
                        let lib_path = Path::new(lib);
                        let processed_lib =
                            if lib_path.has_root() || lib.contains("/") || lib.contains("\\") {
                                // 带路径的库，直接使用完整路径
                                lib.to_string()
                            } else {
                                // 不带路径的库，处理前缀
                                if lib.starts_with("lib") {
                                    // 去掉lib前缀，添加-l
                                    format!("-l{}", &lib[3..])
                                } else {
                                    // 直接添加-l
                                    format!("-l{}", lib)
                                }
                            };
                        // 添加到Build/Target/Linker库列表
                        build_target_linker_libs.push(processed_lib.clone());
                        // 添加到集合用于去重
                        build_target_lib_set.insert(processed_lib);
                    }
                    if let Some(dir) = add.attribute("directory") {
                        // 处理链接库目录，添加-L前缀
                        linker_lib_dirs.push(format!("-L{}", dir));
                    }
                }
            }

            // 处理Target下的Compiler节点，获取宏定义和编译选项
            if let Some(compiler_node) = target_node
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
        }
    }

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

    // 解析Project/Linker节点
    if let Some(linker_node) = project.children().find(|n| n.tag_name().name() == "Linker") {
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
                let processed_lib =
                    if lib_path.has_root() || lib.contains("/") || lib.contains("\\") {
                        // 带路径的库，直接使用完整路径
                        lib.to_string()
                    } else {
                        // 不带路径的库，处理前缀
                        if lib.starts_with("lib") {
                            // 去掉lib前缀，添加-l
                            format!("-l{}", &lib[3..])
                        } else {
                            // 直接添加-l
                            format!("-l{}", lib)
                        }
                    };
                // 只有当Build/Target/Linker中没有这个库时，才添加到Project/Linker库列表
                if !build_target_lib_set.contains(&processed_lib) {
                    linker_libs.push(processed_lib);
                }
            }
            if let Some(dir) = add.attribute("directory") {
                linker_lib_dirs.push(format!("-L{}", dir));
            }
        }
    }

    // 合并Project/Linker库和Build/Target/Linker库，Build/Target/Linker库放最后
    linker_libs = [linker_libs, build_target_linker_libs].concat();

    let options_str = global_cflags.join(" ");
    let includes_str = include_dirs.join(" ");

    let toolchain = ToolchainConfig::from_compiler_id(&compiler_id)
        .unwrap_or_else(|| {
            // 如果找不到对应的编译器ID，回退到默认值，这里保持与 main.rs 一致的逻辑
            ToolchainConfig::from_compiler_id("riscv32-v2").unwrap()
        });
    
    // 获取编译器的执行路径 (例如: C:\Program Files\...\riscv32-elf-gcc.exe)
    // 这样生成的 bat 文件中可以直接调用绝对路径，避免依赖 PATH 环境变量
    let compiler_cmd = format!("\"{}\"", toolchain.compiler_path());

    // 定义宏替换闭包
    let replace_cb_macros = |cmd: &str| -> String {
        let mut processed = cmd.to_string();
        
        // 1. 替换编译器变量 (现在使用的是 config.rs 中定义的真实路径)
        processed = processed.replace("$compiler", &compiler_cmd);
        
        // 2. 替换编译选项和头文件路径
        processed = processed.replace("$options", &options_str);
        processed = processed.replace("$includes", &includes_str);
        
        // 3. 替换项目信息
        processed = processed.replace("$(PROJECT_NAME)", &project_name);
        
        // 4. 替换项目路径 $(PROJECT_DIR)
        // Code::Blocks 中 $(PROJECT_DIR) 通常指 .cbp 文件所在目录
        // 在生成的批处理中，我们通常在项目根目录运行，所以替换为当前目录
        if processed.contains("$(PROJECT_DIR)") {
            // 替换为 Windows 风格的当前目录引用，或者根据 cmd 上下文调整
            // 这里简单的替换为 .\\ 即可，因为后续通常接相对路径
            processed = processed.replace("$(PROJECT_DIR)", ".\\");
        }

        // 5. 额外清理：有时候路径中会出现双反斜杠或混合斜杠，虽然 Windows 通常能容忍，但看着不整洁
        // processed = processed.replace("\\\\", "\\"); 

        processed
    };

    // 解析ExtraCommands节点
    if let Some(extra_commands_node) = project
        .children()
        .find(|n| n.tag_name().name() == "ExtraCommands")
    {
        for add in extra_commands_node
            .children()
            .filter(|n| n.tag_name().name() == "Add")
        {
            if let Some(before) = add.attribute("before") {
                let trimmed_before = before.trim();
                if !trimmed_before.is_empty() {
                    // 应用宏替换
                    let final_cmd = replace_cb_macros(trimmed_before);
                    prebuild_commands.push(final_cmd);
                }
            }
            if let Some(after) = add.attribute("after") {
                let trimmed_after = after.trim();
                if !trimmed_after.is_empty() {
                    // 应用宏替换
                    let final_cmd = replace_cb_macros(trimmed_after);
                    postbuild_commands.push(final_cmd);
                }
            }
        }
    }

    // === 源文件和特殊文件 ===
    let mut source_files = Vec::new();
    let mut special_files = Vec::new();
    let valid_exts: HashSet<&str> = ["c", "cpp", "C", "CPP", "S", "s"].iter().cloned().collect();

    for unit in project.children().filter(|n| n.tag_name().name() == "Unit") {
        if let Some(filename) = unit.attribute("filename") {
            let path = std::path::Path::new(filename);
            let ext = path.extension().and_then(|e| e.to_str());

            // 检查是否是普通源文件
            let is_regular_source = ext.map(|e| valid_exts.contains(e)).unwrap_or(false);

            // 检查是否有编译选项
            let mut should_compile = false;
            let mut build_commands = Vec::new();

            for option in unit.children().filter(|n| n.tag_name().name() == "Option") {
                // 检查是否有compile="1"属性
                if let Some(compile) = option.attribute("compile") {
                    if compile == "1" {
                        should_compile = true;
                    }
                }

                // 检查是否有buildCommand属性和compiler属性
                if let (Some(compiler), Some(build_cmd)) = (
                    option.attribute("compiler"),
                    option.attribute("buildCommand"),
                ) {
                    // 检查是否use="1"且buildCommand不为空
                    if option.attribute("use").unwrap_or("0") == "1" {
                        let trimmed_build_cmd = build_cmd.trim();
                        if !trimmed_build_cmd.is_empty() {
                            build_commands
                                .push((compiler.to_string(), trimmed_build_cmd.to_string()));
                        }
                    }
                }
            }

            if is_regular_source {
                // 普通源文件，添加到source_files
                source_files.push(filename.to_string());
            } else if should_compile && !build_commands.is_empty() {
                // 特殊文件，有编译选项和构建命令
                // 查找匹配当前编译器的构建命令
                let matching_build_cmd = build_commands
                    .iter()
                    .find(|(compiler, _)| compiler == &compiler_id)
                    .or_else(|| build_commands.first());

                if let Some((compiler, build_cmd)) = matching_build_cmd {
                    special_files.push(SpecialFileBuildInfo {
                        filename: filename.to_string(),
                        compiler_id: compiler.clone(),
                        build_command: build_cmd.clone(),
                    });
                }
            }
        }
    }

    if source_files.is_empty() && special_files.is_empty() {
        return Err("No source files (.c/.cpp) or special files found in project.".into());
    }

    // === 解析object_output目录和output文件 ===
    let mut object_output = String::new();
    let mut output = String::new();

    // 查找Build节点
    for build_node in project
        .children()
        .filter(|n| n.tag_name().name() == "Build")
    {
        // 查找Target节点
        for target_node in build_node
            .children()
            .filter(|n| n.tag_name().name() == "Target")
        {
            // 查找带有object_output和output属性的Option节点
            for option_node in target_node
                .children()
                .filter(|n| n.tag_name().name() == "Option")
            {
                if let Some(obj_output) = option_node.attribute("object_output") {
                    object_output = obj_output.to_string();
                }
                if let Some(out) = option_node.attribute("output") {
                    output = out.to_string();
                }
            }
            // 找到一个就够了，跳出循环
            if !object_output.is_empty() && !output.is_empty() {
                break;
            }
        }
        // 找到一个就够了，跳出循环
        if !object_output.is_empty() && !output.is_empty() {
            break;
        }
    }

    // 如果没有找到object_output，使用默认值
    if object_output.is_empty() {
        object_output = "./".to_string();
    }
    // 如果没有找到output，使用默认值
    if output.is_empty() {
        output = format!("{}.elf", project_name);
    }

    Ok(ProjectInfo {
        compiler_id,
        project_name,
        global_cflags,
        include_dirs,
        source_files,
        special_files,
        prebuild_commands,
        postbuild_commands,
        march_info,
        object_output,
        output,
        linker_options,
        linker_libs,
        linker_lib_dirs,
        linker_type: "gcc".to_string(),
    })
}
