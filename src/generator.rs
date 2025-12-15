use crate::config::ToolchainConfig;
use crate::models::CompileCommand;
use crate::parser::ProjectInfo;
use crate::utils::{debug_println, get_short_path};
use std::path::Path;

/// 生成clangd配置文件内容
pub fn generate_clangd_config(
    project_info: &ProjectInfo,
    toolchain: &ToolchainConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    debug_println!("[DEBUG generator] Starting to generate .clangd config...");

    debug_println!("[DEBUG generator] Getting include paths from toolchain...");
    let includes = toolchain
        .include_paths()
        .iter()
        .map(|p| {
            let inc = format!("-I{}", p);
            debug_println!("[DEBUG generator] Added include path: {}", inc);
            inc
        })
        .collect::<Vec<_>>();

    // 构建Add部分
    debug_println!("[DEBUG generator] Building Add flags section...");
    let mut add_flags = vec!["-xc", "-target", "riscv32-unknown-elf"];
    debug_println!("[DEBUG generator] Added base flags: -xc, -target, riscv32-unknown-elf");

    // 添加include路径
    debug_println!("[DEBUG generator] Adding include paths to Add flags...");
    for inc in &includes {
        add_flags.push(&inc[..]);
    }

    debug_println!("[DEBUG generator] Checking for custom march extensions...");
    if project_info.march_info.has_custom_extension {
        debug_println!("[DEBUG generator] Found custom extension, adding base march...");
        if let Some(base_march) = &project_info.march_info.base_march {
            debug_println!("[DEBUG generator] Adding base march: {}", base_march);
            add_flags.push(base_march.as_str());
        }
    }

    // 构建Remove部分
    debug_println!("[DEBUG generator] Building Remove flags section...");
    let mut remove_flags = Vec::new();
    // 如果有-march指令，添加到Remove部分
    if !project_info.march_info.full_march.is_empty() {
        debug_println!(
            "[DEBUG generator] Adding full march to Remove: {}",
            project_info.march_info.full_march
        );
        remove_flags.push(&project_info.march_info.full_march[..]);
    }
    debug_println!("[DEBUG generator] Adding -mjump-tables-in-text to Remove");
    remove_flags.push("-mjump-tables-in-text");

    // 注意：.clangd 是 YAML，但 clangd 接受这种简写格式
    debug_println!("[DEBUG generator] Formatting clangd config content...");
    let mut content = format!("CompileFlags:\n  Add:\n");

    // 添加Add部分
    debug_println!("[DEBUG generator] Adding Add flags to config...");
    for flag in add_flags {
        let formatted_flag = format!("    - {}\n", flag.replace('\\', "\\\\"));
        debug_println!("[DEBUG generator] Added flag: {}", formatted_flag.trim());
        content.push_str(&formatted_flag);
    }

    // 如果有Remove部分，添加
    if !remove_flags.is_empty() {
        debug_println!("[DEBUG generator] Adding Remove flags to config...");
        content.push_str("  Remove:\n");
        for flag in remove_flags {
            debug_println!("[DEBUG generator] Added remove flag: {}", flag);
            content.push_str(&format!("    - {}\n", flag));
        }
    }

    debug_println!("[DEBUG generator] Successfully generated .clangd config content");
    Ok(content)
}

/// 生成编译命令列表
pub fn generate_compile_commands(
    project_info: &crate::parser::ProjectInfo,
    project_dir: &Path,
    toolchain: &ToolchainConfig,
) -> Vec<CompileCommand> {
    debug_println!("[DEBUG generator] Starting to generate compile commands...");
    debug_println!(
        "[DEBUG generator] Project directory: {}",
        project_dir.display()
    );

    // 使用工具链中的编译器路径，但如果路径不存在，使用占位符
    debug_println!("[DEBUG generator] Getting compiler path from toolchain...");
    let compiler_path = toolchain.compiler_path();
    debug_println!("[DEBUG generator] Raw compiler path: {}", compiler_path);
    let compiler_exists = std::path::Path::new(&compiler_path).exists();
    debug_println!(
        "[DEBUG generator] Compiler exists at path: {}",
        compiler_exists
    );

    let compiler = if compiler_exists {
        debug_println!("[DEBUG generator] Compiler exists, attempting to get short path...");
        // 只有当编译器存在时，才尝试获取短路径名
        match get_short_path(&compiler_path) {
            Ok(short_path) => {
                debug_println!(
                    "[DEBUG generator] Successfully got short path: {}",
                    short_path
                );
                short_path
            }
            Err(e) => {
                println!(
                    "[WARNING generator] Failed to get short path for compiler: {}. Using original path.",
                    e
                );
                // 如果失败，使用长文件名路径
                let long_path = format!("\\\\?\\{}", compiler_path);
                debug_println!("[DEBUG generator] Using long path format: {}", long_path);
                long_path
            }
        }
    } else {
        // 如果编译器不存在，使用简单的编译器名称作为占位符
        println!(
            "[WARNING generator] Compiler path {} does not exist. Using placeholder.",
            compiler_path
        );
        "riscv32-elf-gcc".to_string()
    };
    debug_println!("[DEBUG generator] Final compiler path to use: {}", compiler);

    debug_println!("[DEBUG generator] Building base compiler flags...");
    let base_flags: Vec<String> = project_info
        .global_cflags
        .iter()
        .cloned()
        .chain(project_info.include_dirs.iter().cloned())
        .collect();
    debug_println!("[DEBUG generator] Base flags: {:?}", base_flags);

    debug_println!(
        "[DEBUG generator] Starting to process {} source files...",
        project_info.source_files.len()
    );
    let mut compile_commands = Vec::new();
    for (index, src) in project_info.source_files.iter().enumerate() {
        debug_println!(
            "[DEBUG generator] Processing file {}/{}: {}",
            index + 1,
            project_info.source_files.len(),
            src
        );
        // 尝试获取源文件的短路径名
        debug_println!("[DEBUG generator] Attempting to get short path for source file...");
        let src_path = match get_short_path(src) {
            Ok(short_path) => {
                debug_println!(
                    "[DEBUG generator] Successfully got short path: {}",
                    short_path
                );
                short_path
            }
            Err(e) => {
                println!(
                    "[WARNING generator] Failed to get short path for source file {}: {}. Using original path.",
                    src, e
                );
                src.clone()
            }
        };

        debug_println!("[DEBUG generator] Building command parts for file...");
        let mut cmd = vec![&compiler[..], "-c"];
        cmd.extend(base_flags.iter().map(|s| s.as_str()));
        cmd.push(&src_path);

        let command_str = cmd.join(" ");
        debug_println!("[DEBUG generator] Generated command: {}", command_str);

        debug_println!("[DEBUG generator] Creating compile command entry...");
        compile_commands.push(CompileCommand {
            directory: project_dir.to_string_lossy().into_owned(),
            command: command_str,
            file: src.clone(), // 保留原始文件名用于引用
        });
    }

    debug_println!(
        "[DEBUG generator] Successfully generated {} compile commands",
        compile_commands.len()
    );
    compile_commands
}

/// 生成ninja构建文件内容
pub fn generate_ninja_build(
    project_info: &ProjectInfo,
    project_dir: &Path,
    toolchain: &ToolchainConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    debug_println!("[DEBUG generator] Starting to generate ninja build file...");

    // 使用工具链中的编译器路径，但如果路径不存在，使用占位符
    let compiler_path = toolchain.compiler_path();
    let compiler_exists = std::path::Path::new(&compiler_path).exists();
    let compiler = if compiler_exists {
        match get_short_path(&compiler_path) {
            Ok(short_path) => short_path,
            Err(e) => {
                println!(
                    "[WARNING generator] Failed to get short path for compiler: {}. Using original path.",
                    e
                );
                let long_path = format!("\\?\\{}", compiler_path);
                long_path
            }
        }
    } else {
        println!(
            "[WARNING generator] Compiler path {} does not exist. Using placeholder.",
            compiler_path
        );
        "riscv32-elf-gcc".to_string()
    };
    
    // 获取链接器路径
    let linker_path = toolchain.linker_path(&project_info.linker_type);
    let linker_exists = std::path::Path::new(&linker_path).exists();
    let linker = if linker_exists {
        match get_short_path(&linker_path) {
            Ok(short_path) => short_path,
            Err(e) => {
                println!(
                    "[WARNING generator] Failed to get short path for linker: {}. Using original path.",
                    e
                );
                let long_path = format!("\\?\\{}", linker_path);
                long_path
            }
        }
    } else {
        println!(
            "[WARNING generator] Linker path {} does not exist. Using placeholder.",
            linker_path
        );
        if project_info.linker_type == "ld" {
            "riscv32-elf-ld".to_string()
        } else {
            "riscv32-elf-gcc".to_string()
        }
    };

    // 构建基础编译器标志
    let mut base_flags: Vec<&str> = Vec::new();
    for flag in &project_info.global_cflags {
        base_flags.push(flag.as_str());
    }
    for include in &project_info.include_dirs {
        base_flags.push(include.as_str());
    }



    // 规则部分
    let mut ninja_content = String::new();
    ninja_content.push_str("# Generated by cbp2clangd\n");
    ninja_content.push_str("\n");
    ninja_content.push_str("rule cc\n");
    ninja_content.push_str(&format!("  command = {} $flags -c $in -o $out\n", compiler));
    ninja_content.push_str("  depfile = $out.d\n");
    ninja_content.push_str("  deps = gcc\n");
    ninja_content.push_str("\n");
    ninja_content.push_str("rule link\n");
    ninja_content.push_str(&format!("  command = {} $flags $in -o $out\n", linker));
    ninja_content.push_str("\n");

    // 构建对象文件列表，分为普通对象文件和特殊对象文件
    let mut regular_obj_files = Vec::new();
    
    // 处理普通源文件
    for src in &project_info.source_files {
        let src_path = Path::new(src);
        let src_stem = src_path.file_stem().unwrap_or_else(|| std::ffi::OsStr::new(""));
        
        // 提取源文件的目录结构
        let src_dir = src_path.parent().unwrap_or_else(|| std::path::Path::new(""));
        
        // 构建对象文件的完整路径：object_output + 源文件目录 + 源文件名.o
        let obj_name = if src_dir == std::path::Path::new("") {
            // 源文件在根目录，直接使用文件名
            format!("{}{}.o", project_info.object_output, src_stem.to_string_lossy())
        } else {
            // 源文件在子目录，保留目录结构
            let full_path = Path::new(&project_info.object_output)
                .join(src_dir)
                .join(format!("{}.o", src_stem.to_string_lossy()));
            full_path.to_string_lossy().to_string()
        };
        regular_obj_files.push(obj_name);
    }
    
    // 处理特殊文件（只编译，不链接）
    for special_file in &project_info.special_files {
        // 解析构建命令中的目标文件名
        let mut processed_cmd = special_file.build_command.clone();
        
        // 替换变量
        processed_cmd = processed_cmd.replace("$compiler", &compiler);
        processed_cmd = processed_cmd.replace("$options", &base_flags.join(" "));
        processed_cmd = processed_cmd.replace("$includes", &project_info.include_dirs.join(" "));
        processed_cmd = processed_cmd.replace("$file", &special_file.filename);
        processed_cmd = processed_cmd.replace("$(TARGET_OBJECT_DIR)", &project_info.object_output);
        processed_cmd = processed_cmd.replace("$(TARGET_OUTPUT_DIR)", &project_info.object_output);
        
        // 提取输出文件名
        let output_file = if let Some(output_pos) = processed_cmd.find("-o ") {
            let rest = &processed_cmd[output_pos + 3..];
            // 找到下一个空格或行尾
            if let Some(space_pos) = rest.find(' ') {
                rest[..space_pos].to_string()
            } else {
                rest.to_string()
            }
        } else {
            // 如果没有找到-o选项，使用默认的输出文件名
            let src_path = Path::new(&special_file.filename);
            let src_stem = src_path.file_stem().unwrap_or_else(|| std::ffi::OsStr::new(""));
            format!("{}{}.o", project_info.object_output, src_stem.to_string_lossy())
        };
        
        // 生成特殊文件的构建规则
        let rule_name = format!("special_{}", special_file.filename.replace(".", "_").replace("/", "_").replace("\\", "_"));
        ninja_content.push_str(&format!("rule {}\n", rule_name));
        ninja_content.push_str(&format!("  command = {}\n", processed_cmd));
        ninja_content.push_str("\n");
        
        // 生成构建规则
        ninja_content.push_str(&format!("build {}: {} {}\n", output_file, rule_name, special_file.filename));
        ninja_content.push_str("\n");
    }

    // 构建部分 - 普通源文件
    for (src, obj) in project_info.source_files.iter().zip(regular_obj_files.iter()) {
        ninja_content.push_str(&format!("build {}: cc {}\n", obj, src));
        ninja_content.push_str(&format!("  flags = {}\n", base_flags.join(" ")));
        ninja_content.push_str("\n");
    }

    // 链接目标 - 只使用普通对象文件，不包含特殊文件
    let project_name = project_dir
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("output"))
        .to_string_lossy();
    let target_name = format!("{}.elf", project_name);
    
    // 构建链接标志
    let mut link_flags: Vec<String> = Vec::new();
    for opt in &project_info.linker_options {
        // 替换 $(TARGET_OBJECT_DIR) 为实际的 object_output 路径
        let replaced_opt = opt.replace("$(TARGET_OBJECT_DIR)", &project_info.object_output);
        link_flags.push(replaced_opt);
    }
    for lib_dir in &project_info.linker_lib_dirs {
        link_flags.push(lib_dir.clone());
    }
    for lib in &project_info.linker_libs {
        link_flags.push(lib.clone());
    }
    
    ninja_content.push_str(&format!("build {}: link {}\n", target_name, regular_obj_files.join(" ")));
    if !link_flags.is_empty() {
        ninja_content.push_str(&format!("  flags = {}\n", link_flags.join(" ")));
    }
    ninja_content.push_str("\n");

    // 默认目标
    ninja_content.push_str(&format!("default {}\n", target_name));

    debug_println!("[DEBUG generator] Successfully generated ninja build file content");
    Ok(ninja_content)
}

/// 生成构建脚本文件内容
pub fn generate_build_script(
    project_info: &ProjectInfo,
    toolchain: &ToolchainConfig,
    project_dir: &Path,
) -> String {
    debug_println!("[DEBUG generator] Starting to generate build script...");
    
    let mut script_content = String::new();
    
    // 1. 添加工具链路径到PATH环境变量
    let toolchain_bin = format!("{}\\bin", toolchain.get_base_path());
    script_content.push_str("@echo off\n");
    script_content.push_str("rem Generated by cbp2clangd\n");
    script_content.push_str("\n");
    script_content.push_str("rem Set toolchain path\n");
    script_content.push_str(&format!("set PATH={};%PATH%\n", toolchain_bin));
    script_content.push_str("\n");
    
    // 2. 添加预构建命令
    if !project_info.prebuild_commands.is_empty() {
        script_content.push_str("rem Prebuild commands\n");
        for cmd in &project_info.prebuild_commands {
            // 替换 $(PROJECT_NAME) 变量
            let processed_cmd = cmd.replace("$(PROJECT_NAME)", &project_info.project_name);
            script_content.push_str(&format!("{}\n", processed_cmd));
        }
        script_content.push_str("\n");
    }
    
    // 3. 添加ninja构建命令
    script_content.push_str("rem Build project with ninja\n");
    script_content.push_str("ninja -f build.ninja\n");
    script_content.push_str("if %errorlevel% neq 0 exit /b %errorlevel%\n");
    script_content.push_str("\n");
    
    // 4. 添加后构建命令
    if !project_info.postbuild_commands.is_empty() {
        script_content.push_str("rem Postbuild commands\n");
        for cmd in &project_info.postbuild_commands {
            // 替换 $(PROJECT_NAME) 变量
            let processed_cmd = cmd.replace("$(PROJECT_NAME)", &project_info.project_name);
            script_content.push_str(&format!("{}\n", processed_cmd));
        }
        script_content.push_str("\n");
    }
    
    // 5. 添加完成信息
    script_content.push_str("rem Build completed successfully\n");
    script_content.push_str("echo Build completed successfully\n");
    
    debug_println!("[DEBUG generator] Successfully generated build script content");
    script_content
}
