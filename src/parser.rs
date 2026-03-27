use crate::ToolchainConfig;
use crate::models::{BuildTarget, SpecialFileBuildInfo, SourceFileInfo};
use roxmltree::Document;
use std::collections::HashSet;
use std::path::Path;

/// 项目信息结构
pub struct ProjectInfo {
    pub compiler_id: String,
    pub project_name: String,
    pub global_cflags: Vec<String>,          // 全局编译选项 (Project/Compiler)
    pub global_include_dirs: Vec<String>,    // 全局头文件目录 (Project/Compiler)
    pub global_linker_libs: Vec<String>,     // 全局链接库 (Project/Linker)
    pub global_linker_options: Vec<String>,  // 全局链接器选项 (Project/Linker)
    pub source_files: Vec<SourceFileInfo>,
    pub special_files: Vec<SpecialFileBuildInfo>,
    pub prebuild_commands: Vec<String>,
    pub postbuild_commands: Vec<String>,
    pub targets: Vec<BuildTarget>,           // 各个Build Target的配置
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

    // === 全局编译选项 (Project/Compiler) ===
    let mut global_cflags = Vec::new();
    let mut global_include_dirs = Vec::new();
    let mut global_linker_libs = Vec::new();

    // 解析Project级别Compiler节点
    if let Some(compiler_node) = project
        .children()
        .find(|n| n.tag_name().name() == "Compiler")
    {
        for add in compiler_node
            .children()
            .filter(|n| n.tag_name().name() == "Add")
        {
            if let Some(opt) = add.attribute("option") {
                global_cflags.push(opt.to_string());
            }
            if let Some(dir) = add.attribute("directory") {
                global_include_dirs.push(format!("-I{}", dir));
            }
        }
    }

    // 解析Project级别Linker节点
    let mut global_linker_options = Vec::new();
    if let Some(linker_node) = project.children().find(|n| n.tag_name().name() == "Linker") {
        for add in linker_node
            .children()
            .filter(|n| n.tag_name().name() == "Add")
        {
            if let Some(lib) = add.attribute("library") {
                let processed_lib = process_library_path(lib);
                global_linker_libs.push(processed_lib);
            }
            if let Some(opt) = add.attribute("option") {
                global_linker_options.push(opt.to_string());
            }
        }
    }

    // === 解析Build Targets ===
    let mut targets = Vec::new();
    let mut prebuild_commands = Vec::new();
    let mut postbuild_commands = Vec::new();

    for build_node in project
        .children()
        .filter(|n| n.tag_name().name() == "Build")
    {
        for target_node in build_node
            .children()
            .filter(|n| n.tag_name().name() == "Target")
        {
            let target_name = target_node.attribute("title").unwrap_or("Default").to_string();
            println!("Found target: {}", target_name);

            let mut target = BuildTarget {
                name: target_name,
                ..Default::default()
            };

            // 解析Target级别的Option (output, object_output等)
            for option_node in target_node.children().filter(|n| n.tag_name().name() == "Option") {
                if let Some(output) = option_node.attribute("output") {
                    target.output = output.to_string();
                }
                if let Some(obj_output) = option_node.attribute("object_output") {
                    target.object_output = obj_output.to_string();
                }
            }

            // 解析Target级别Compiler
            if let Some(compiler_node) = target_node.children().find(|n| n.tag_name().name() == "Compiler") {
                for add in compiler_node.children().filter(|n| n.tag_name().name() == "Add") {
                    if let Some(opt) = add.attribute("option") {
                        let opt_str = opt.to_string();
                        target.cflags.push(opt_str.clone());

                        // 检测宏定义 (-D)
                        if opt_str.starts_with("-D") {
                            target.defines.push(opt_str.clone());
                        }

                        // 检测并解析-march=指令
                        if opt_str.starts_with("-march=") {
                            let march_value = opt_str.trim_start_matches("-march=");
                            target.march_info.full_march = opt_str.clone();

                            if let Some(x_index) = march_value.find('x') {
                                let base_part = march_value[0..x_index].trim_end_matches('_');
                                if !base_part.is_empty() {
                                    target.march_info.base_march = Some(format!("-march={}", base_part));
                                    target.march_info.has_custom_extension = true;
                                }
                            }
                        }
                    }
                    if let Some(dir) = add.attribute("directory") {
                        target.include_dirs.push(format!("-I{}", dir));
                    }
                }
            }

            // 解析Target级别Linker
            if let Some(linker_node) = target_node.children().find(|n| n.tag_name().name() == "Linker") {
                for add in linker_node.children().filter(|n| n.tag_name().name() == "Add") {
                    if let Some(opt) = add.attribute("option") {
                        target.linker_options.push(opt.to_string());
                    }
                    if let Some(lib) = add.attribute("library") {
                        target.linker_libs.push(process_library_path(lib));
                    }
                    if let Some(dir) = add.attribute("directory") {
                        target.linker_lib_dirs.push(format!("-L{}", dir));
                    }
                }
            }

            // 如果没有找到output，使用默认值
            if target.output.is_empty() {
                target.output = format!("{}.elf", project_name);
            }

            // 如果没有找到object_output，从output推导
            if target.object_output.is_empty() {
                let output_path = Path::new(&target.output);
                if let Some(parent) = output_path.parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    if parent_str.is_empty() {
                        target.object_output = "./".to_string();
                    } else {
                        target.object_output = parent_str;
                        if !target.object_output.ends_with('/') && !target.object_output.ends_with('\\') {
                            target.object_output.push(std::path::MAIN_SEPARATOR);
                        }
                    }
                } else {
                    target.object_output = "./".to_string();
                }
            }

            targets.push(target);
        }
    }

    // 如果没有找到任何target，创建一个默认的
    if targets.is_empty() {
        targets.push(BuildTarget {
            name: "Default".to_string(),
            output: format!("{}.elf", project_name),
            object_output: "./".to_string(),
            ..Default::default()
        });
    }

    // 对每个编译选项和include路径进行引号处理，防止空格导致命令解析错误
    let quoted_global_cflags: Vec<_> = global_cflags.iter().map(|opt| {
        if opt.contains(' ') {
            format!("\"{}\"", opt)
        } else {
            opt.clone()
        }
    }).collect();
    let quoted_include_dirs: Vec<_> = global_include_dirs.iter().map(|dir| {
        if dir.contains(' ') {
            format!("\"{}\"", dir)
        } else {
            dir.clone()
        }
    }).collect();

    let options_str = quoted_global_cflags.join(" ");
    let includes_str = quoted_include_dirs.join(" ");

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

            // 初始化编译和链接标志
            // 普通源文件：默认编译，默认链接
            // 特殊文件：需要明确指定compile="1"才编译，默认不链接
            let mut compile = is_regular_source;
            let mut link = is_regular_source;
            let mut build_commands = Vec::new();

            for option in unit.children().filter(|n| n.tag_name().name() == "Option") {
                // 检查compile属性：0关闭，1开启
                if let Some(compile_attr) = option.attribute("compile") {
                    compile = compile_attr == "1";
                }

                // 检查link属性：0关闭，1开启
                if let Some(link_attr) = option.attribute("link") {
                    link = link_attr == "1";
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

            // 处理普通源文件
            if is_regular_source {
                // 普通源文件：根据compile和link属性决定是否编译和链接
                source_files.push(SourceFileInfo {
                    filename: filename.to_string(),
                    compile,
                    link,
                });
            } else {
                // 处理特殊文件
                // 查找匹配当前编译器的构建命令
                let matching_build_cmd = build_commands
                    .iter()
                    .find(|(compiler, _)| compiler == &compiler_id)
                    .or_else(|| build_commands.first());

                let (compiler_id, build_command) = if let Some((compiler, build_cmd)) = matching_build_cmd {
                    (compiler.clone(), build_cmd.clone())
                } else {
                    // 没有匹配的构建命令，使用默认值
                    (compiler_id.clone(), String::new())
                };

                // 只处理有意义的特殊文件，忽略头文件等
                let is_header_file = ext.map(|e| e.to_lowercase() == "h" || e.to_lowercase() == "hpp").unwrap_or(false);
                if !is_header_file {
                    special_files.push(SpecialFileBuildInfo {
                        filename: filename.to_string(),
                        compiler_id,
                        build_command,
                        compile,
                        link,
                    });
                }
            }
        }
    }

    if source_files.is_empty() && special_files.is_empty() {
        return Err("No source files (.c/.cpp) or special files found in project.".into());
    }

    Ok(ProjectInfo {
        compiler_id,
        project_name,
        global_cflags,
        global_include_dirs,
        global_linker_libs,
        global_linker_options,
        source_files,
        special_files,
        prebuild_commands,
        postbuild_commands,
        targets,
        linker_type: "gcc".to_string(),
    })
}

/// 处理库路径，添加适当的前缀
fn process_library_path(lib: &str) -> String {
    let lib_path = Path::new(lib);
    if lib_path.has_root() || lib.contains('/') || lib.contains('\\') {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 一个最小化的测试用 CBP 内容
    const TEST_XML: &str = r#"
    <CodeBlocks_project_file>
        <FileVersion major="1" minor="6" />
        <Project>
            <Option title="TestProject" />
            <Option compiler="riscv32-v2" />
            <Build>
                <Target title="Debug">
                    <Option output="bin/Debug/TestProject.elf" />
                    <Option object_output="obj/Debug/" />
                    <Compiler>
                        <Add option="-g" />
                    </Compiler>
                    <Linker>
                        <Add library="m" />
                    </Linker>
                </Target>
            </Build>
            <Compiler>
                <Add option="-Wall" />
                <Add directory="src/include" />
            </Compiler>
            <Unit filename="main.c">
                <Option compilerVar="CC" />
            </Unit>
            <Unit filename="utils.c">
                <Option compilerVar="CC" />
            </Unit>
        </Project>
    </CodeBlocks_project_file>
    "#;

    #[test]
    fn test_parse_basic_project() {
        let project = parse_cbp_file(TEST_XML).expect("Failed to parse valid XML");

        // 验证基本信息
        assert_eq!(project.project_name, "TestProject");
        assert_eq!(project.compiler_id, "riscv32-v2");

        // 验证源文件列表
        assert!(project.source_files.iter().any(|f| f.filename == "main.c"));
        assert!(project.source_files.iter().any(|f| f.filename == "utils.c"));

        // 验证全局 include 路径 (注意解析器里添加了 -I 前缀)
        assert!(project.global_include_dirs.contains(&"-Isrc/include".to_string()));

        // 验证全局 Flag
        assert!(project.global_cflags.contains(&"-Wall".to_string()));

        // 验证 Targets
        assert_eq!(project.targets.len(), 1);
        let debug_target = &project.targets[0];
        assert_eq!(debug_target.name, "Debug");
        assert_eq!(debug_target.output, "bin/Debug/TestProject.elf");
        assert_eq!(debug_target.object_output, "obj/Debug/");

        // 验证 Target 级别的编译选项
        assert!(debug_target.cflags.contains(&"-g".to_string()));

        // 验证 Target 级别的链接库
        assert!(debug_target.linker_libs.iter().any(|l| l.contains("m")));
    }

    #[test]
    fn test_parse_march_extension() {
        // 这里的 XML 必须包含至少一个 Unit，否则 parse_cbp_file 会报错 "No source files..."
        let xml = r#"
        <CodeBlocks_project_file>
            <Project>
                <Build>
                    <Target title="Debug">
                        <Compiler>
                            <Add option="-march=rv32imac_xabcd" />
                        </Compiler>
                    </Target>
                </Build>
                <Unit filename="dummy.c">
                    <Option compile="1" />
                </Unit>
            </Project>
        </CodeBlocks_project_file>
        "#;

        // 现在这里应该返回 Ok，而不是 Err
        let project = parse_cbp_file(xml).expect("Failed to parse project with custom march");

        // march_info 现在在 target 中
        assert_eq!(project.targets.len(), 1);
        let target = &project.targets[0];
        assert_eq!(target.march_info.full_march, "-march=rv32imac_xabcd");
        assert!(target.march_info.has_custom_extension);
        assert_eq!(target.march_info.base_march, Some("-march=rv32imac".to_string()));
    }

    #[test]
    fn test_parse_object_output_from_output_dir() {
        let xml = r#"
        <CodeBlocks_project_file>
            <FileVersion major="1" minor="6" />
            <Project>
                <Option title="TestProject" />
                <Option compiler="riscv32-v2" />
                <Build>
                    <Target title="Debug">
                        <Option output="bin/Debug/TestProject.elf" />
                        <Option object_output="obj/Debug/" />
                        <Compiler>
                            <Add option="-g" />
                        </Compiler>
                        <Linker>
                            <Add library="m" />
                        </Linker>
                    </Target>
                </Build>
                <Compiler>
                    <Add option="-Wall" />
                    <Add directory="src/include" />
                </Compiler>
                <Unit filename="main.c">
                    <Option compilerVar="CC" />
                </Unit>
            </Project>
        </CodeBlocks_project_file>
        "#;

        let project = parse_cbp_file(xml).expect("Failed to parse valid XML");

        // 验证 targets
        assert_eq!(project.targets.len(), 1);
        let target = &project.targets[0];

        // 验证 output 路径
        assert_eq!(target.output, "bin/Debug/TestProject.elf");

        // 验证 object_output 现在应该等于指定的 object_output 值，而不是从 output 推导
        assert_eq!(target.object_output, "obj/Debug/");
    }

    #[test]
    fn test_parse_multiple_targets() {
        let xml = r#"
        <CodeBlocks_project_file>
            <FileVersion major="1" minor="6" />
            <Project>
                <Option title="MultiTargetProject" />
                <Option compiler="riscv32-v2" />
                <Build>
                    <Target title="Debug">
                        <Option output="bin/Debug/app.elf" />
                        <Option object_output="obj/Debug/" />
                        <Compiler>
                            <Add option="-g" />
                            <Add option="-DDEBUG=1" />
                        </Compiler>
                        <Linker>
                            <Add library="m" />
                        </Linker>
                    </Target>
                    <Target title="Release">
                        <Option output="bin/Release/app.elf" />
                        <Option object_output="obj/Release/" />
                        <Compiler>
                            <Add option="-O2" />
                            <Add option="-DNDEBUG=1" />
                        </Compiler>
                    </Target>
                </Build>
                <Unit filename="main.c">
                    <Option compilerVar="CC" />
                </Unit>
            </Project>
        </CodeBlocks_project_file>
        "#;

        let project = parse_cbp_file(xml).expect("Failed to parse valid XML");

        // 验证基本信息
        assert_eq!(project.project_name, "MultiTargetProject");
        assert_eq!(project.targets.len(), 2);

        // 验证 Debug Target
        let debug_target = &project.targets[0];
        assert_eq!(debug_target.name, "Debug");
        assert_eq!(debug_target.output, "bin/Debug/app.elf");
        assert_eq!(debug_target.object_output, "obj/Debug/");
        assert!(debug_target.cflags.contains(&"-g".to_string()));
        assert!(debug_target.defines.contains(&"-DDEBUG=1".to_string()));
        assert!(debug_target.linker_libs.iter().any(|l| l.contains("m")));

        // 验证 Release Target
        let release_target = &project.targets[1];
        assert_eq!(release_target.name, "Release");
        assert_eq!(release_target.output, "bin/Release/app.elf");
        assert_eq!(release_target.object_output, "obj/Release/");
        assert!(release_target.cflags.contains(&"-O2".to_string()));
        assert!(release_target.defines.contains(&"-DNDEBUG=1".to_string()));
    }
}
