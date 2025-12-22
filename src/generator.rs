use crate::config::ToolchainConfig;
use crate::debug_println;
use crate::models::CompileCommand;
use crate::parser::ProjectInfo;
use crate::utils::get_short_path;
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
    let mut add_flags = vec!["-xc", "-target riscv32-unknown-elf"];
    debug_println!("[DEBUG generator] Added base flags: -xc, -target riscv32-unknown-elf");

    // 添加include路径
    debug_println!("[DEBUG generator] Adding include paths to Add flags...");
    for inc in &includes {
        add_flags.push(&inc[..]);
    }

    // 添加全局编译选项（包括宏定义）
    debug_println!("[DEBUG generator] Adding global_cflags to Add flags...");
    for flag in &project_info.global_cflags {
        // 跳过-march选项，因为我们会单独处理
        if flag.starts_with("-march=") {
            debug_println!("[DEBUG generator] Skipping march flag from global_cflags: {}", flag);
            continue;
        }
        // 跳过-mjump-tables-in-text选项，因为我们会将其添加到Remove部分
        if flag == "-mjump-tables-in-text" {
            debug_println!("[DEBUG generator] Skipping jump-tables flag from global_cflags: {}", flag);
            continue;
        }
        debug_println!("[DEBUG generator] Added global flag: {}", flag);
        add_flags.push(flag.as_str());
    }

    debug_println!("[DEBUG generator] Checking for custom march extensions...");
    if project_info.march_info.has_custom_extension {
        debug_println!("[DEBUG generator] Found custom extension, adding base march...");
        if let Some(base_march) = &project_info.march_info.base_march {
            debug_println!("[DEBUG generator] Adding base march: {}", base_march);
            add_flags.push(base_march.as_str());
        }
    } else if !project_info.march_info.full_march.is_empty() {
        // 如果没有自定义扩展，添加完整的-march选项
        debug_println!("[DEBUG generator] No custom extension, adding full march: {}", project_info.march_info.full_march);
        add_flags.push(&project_info.march_info.full_march[..]);
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

/// 尝试解析库文件的实际路径
///
/// 1. 如果 lib 是绝对路径，直接检查
/// 2. 如果 lib 是相对路径，基于 project_dir 检查
/// 3. 如果 lib 是 -lname 格式，在 lib_dirs (基于 project_dir) 中搜索
fn resolve_library_path(lib: &str, lib_dirs: &[String], root_dir: &Path) -> Option<String> {
    // 1. 处理库名称
    let (search_names, is_flag) = if lib.starts_with("-l") {
        let name = &lib[2..];
        // 如果是 -lfoo，则搜索 libfoo.a
        (vec![format!("lib{}.a", name)], true)
    } else {
        // 如果直接是文件名
        if lib.ends_with(".a") || lib.ends_with(".o") {
            (vec![lib.to_string()], false)
        } else {
            (vec![lib.to_string(), format!("lib{}.a", lib)], false)
        }
    };

    // 2. 如果不是 flag (-l)，直接检查文件路径
    if !is_flag {
        let p = Path::new(lib);
        // 如果是绝对路径直接检查，如果是相对路径则拼接 root_dir
        let full_p = if p.is_absolute() {
            p.to_path_buf()
        } else {
            root_dir.join(p)
        };
        
        if full_p.exists() {
            // 返回规范化的路径或者相对路径，为了Ninja文件整洁，这里建议返回相对于root的路径或绝对路径
            // 这里返回 canonicalize 后的路径以确保 Ninja 能绝对找到它
            return full_p.canonicalize().ok().map(|p| p.to_string_lossy().to_string())
                   .or_else(|| Some(full_p.to_string_lossy().to_string()));
        }
    }

    // 3. 在库目录中搜索
    for dir_flag in lib_dirs {
        // 移除 -L 前缀
        let raw_dir = if dir_flag.starts_with("-L") {
            &dir_flag[2..]
        } else {
            dir_flag
        };

        let dir_path = Path::new(raw_dir);
        
        // 解析库目录的实际位置
        let search_dir = if dir_path.is_absolute() {
            dir_path.to_path_buf()
        } else {
            root_dir.join(dir_path)
        };

        // 在该目录下搜索库文件
        for name in &search_names {
            let full_path = search_dir.join(name);
            if full_path.exists() {
                debug_println!("[DEBUG generator] Found lib at: {}", full_path.display());
                // 同样，优先返回规范化的绝对路径，防止路径中包含 .. 导致 Ninja 解析困惑
                return full_path.canonicalize().ok().map(|p| p.to_string_lossy().to_string())
                       .or_else(|| Some(full_path.to_string_lossy().to_string()));
            }
        }
    }

    None
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
    ninja_content.push_str(&format!(
        "  command = {} $flags -MMD -MF $out.d -c $in -o $out\n",
        compiler
    ));
    ninja_content.push_str("  depfile = $out.d\n");
    ninja_content.push_str("  deps = gcc\n");
    ninja_content.push_str("\n");

    // 构建对象文件列表，分为普通对象文件和特殊对象文件
    let mut regular_obj_files = Vec::new();

    // 处理普通源文件
    for src in &project_info.source_files {
        let src_path = Path::new(src);

        // 构建对象文件的完整路径：object_output + 源文件名.o
        // 注意：这里不直接使用src的父目录，而是从src的完整路径中移除..，确保对象文件在output目录内
        let obj_name = {
            // 从src路径中移除父目录引用，确保对象文件在output目录内
            let mut normalized_path = Vec::new();
            for component in src_path.components() {
                match component {
                    std::path::Component::ParentDir => {
                        // 如果不是第一个组件，移除上一个组件
                        if !normalized_path.is_empty() {
                            normalized_path.pop();
                        }
                    }
                    std::path::Component::Normal(component) => {
                        normalized_path.push(component);
                    }
                    _ => {}
                }
            }

            // 构建规范化后的路径
            let mut full_path = Path::new(&project_info.object_output).to_path_buf();
            for component in normalized_path {
                full_path.push(component);
            }
            // 替换文件名后缀为.o
            if let Some(file_name) = full_path.file_name() {
                let file_name_str = file_name.to_string_lossy().to_string();
                let stem = file_name_str.split('.').next().unwrap_or("");
                full_path.set_file_name(format!("{}.o", stem));
            }
            full_path.to_string_lossy().to_string()
        };
        regular_obj_files.push(obj_name);
    }

    // 处理特殊文件（只编译，不链接）
    let mut special_output_files = Vec::new();
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

            // 构建规范化的对象文件路径，确保在output目录内
            let mut normalized_path = Vec::new();
            for component in src_path.components() {
                match component {
                    std::path::Component::ParentDir => {
                        if !normalized_path.is_empty() {
                            normalized_path.pop();
                        }
                    }
                    std::path::Component::Normal(component) => {
                        normalized_path.push(component);
                    }
                    _ => {}
                }
            }

            let mut full_path = Path::new(&project_info.object_output).to_path_buf();
            for component in normalized_path {
                full_path.push(component);
            }
            if let Some(file_name) = full_path.file_name() {
                let file_name_str = file_name.to_string_lossy().to_string();
                let stem = file_name_str.split('.').next().unwrap_or("");
                full_path.set_file_name(format!("{}.o", stem));
            }
            full_path.to_string_lossy().to_string()
        };

        // 添加到特殊输出文件列表
        special_output_files.push(output_file.clone());

        // 生成特殊文件的构建规则
        let rule_name = format!(
            "special_{}",
            special_file
                .filename
                .replace(".", "_")
                .replace("/", "_")
                .replace("\\", "_")
        );
        ninja_content.push_str(&format!("rule {}\n", rule_name));
        ninja_content.push_str(&format!("  command = {}\n", processed_cmd));
        ninja_content.push_str("\n");

        // 生成构建规则
        ninja_content.push_str(&format!(
            "build {}: {} {}\n",
            output_file, rule_name, special_file.filename
        ));
        ninja_content.push_str("\n");
    }

    // 构建部分 - 普通源文件
    for (src, obj) in project_info
        .source_files
        .iter()
        .zip(regular_obj_files.iter())
    {
        ninja_content.push_str(&format!("build {}: cc {}\n", obj, src));
        ninja_content.push_str(&format!("  flags = {}\n", base_flags.join(" ")));
        ninja_content.push_str("\n");
    }

    // 链接目标 - 使用ProjectInfo中的output字段
    let mut target_name = project_info.output.clone();

    // 检查是否为静态库目标（.a结尾）
    let is_static_lib = target_name.ends_with(".a");

    // 处理静态库文件名，添加lib前缀（如果没有的话）
    if is_static_lib {
        let target_path = Path::new(&target_name);
        if let Some(file_name) = target_path.file_name() {
            let file_name_str = file_name.to_string_lossy().to_string();
            if !file_name_str.starts_with("lib") {
                let dir = target_path.parent().unwrap_or_else(|| Path::new("."));
                let stem = file_name_str.strip_suffix(".a").unwrap_or(&file_name_str);
                let new_file_name = format!("lib{}.a", stem);
                target_name = dir.join(new_file_name).to_string_lossy().to_string();
            }
        }
    }

    // 生成主目标的构建规则，确保特殊文件被编译但不被链接
    if is_static_lib {
        // 静态库目标
        // 定义ar规则
        let ar_path = toolchain.ar_path();
        let ar_exists = std::path::Path::new(&ar_path).exists();
        let ar = if ar_exists {
            match get_short_path(&ar_path) {
                Ok(short_path) => short_path,
                Err(e) => {
                    println!(
                        "[WARNING generator] Failed to get short path for ar: {}. Using original path.",
                        e
                    );
                    let long_path = format!("\\?\\{}", ar_path);
                    long_path
                }
            }
        } else {
            println!(
                "[WARNING generator] Ar path {} does not exist. Using placeholder.",
                ar_path
            );
            "riscv32-elf-ar".to_string()
        };

        ninja_content.push_str("rule ar\n");
        ninja_content.push_str(&format!(
            "  command = cmd /c (if exist \"$out\" del /q \"$out\") & {} crs $out $in\n",
            ar
        ));
        ninja_content.push_str("\n");

        // 生成静态库构建规则
        if special_output_files.is_empty() {
            // 没有特殊文件，直接使用普通对象文件
            ninja_content.push_str(&format!(
                "build {}: ar {}\n",
                target_name,
                regular_obj_files.join(" ")
            ));
        } else {
            // 有特殊文件，将它们作为隐式依赖，确保被编译但不被链接
            ninja_content.push_str(&format!(
                "build {}: ar {} | {}\n",
                target_name,
                regular_obj_files.join(" "),
                special_output_files.join(" ")
            ));
        }
    } else {
        // 可执行文件目标
        // 构建链接标志，分为前导标志和库标志
        let mut pre_link_flags: Vec<String> = Vec::new();
        let mut lib_flags: Vec<String> = Vec::new();
        
        // 1. 解析库依赖，寻找实际路径
        let mut resolved_lib_dependencies = Vec::new();

        debug_println!("[DEBUG generator] Resolving library dependencies...");
        debug_println!("[DEBUG generator] Search base dir: {}", project_dir.display()); // 添加调试日志
        
        for lib in &project_info.linker_libs {
            // 将原始标志加入 lib_flags 传递给链接器命令
            lib_flags.push(lib.clone());

            // --- 修改点：传入 project_dir ---
            if let Some(resolved_path) = resolve_library_path(lib, &project_info.linker_lib_dirs, project_dir) {
                debug_println!("[DEBUG generator] Resolved library {} to {}", lib, resolved_path);
                resolved_lib_dependencies.push(resolved_path);
            } else {
                debug_println!("[DEBUG generator] Could not resolve library path for {}", lib);
            }
        }

        // 添加链接器选项
        for opt in &project_info.linker_options {
            // 替换 $(TARGET_OBJECT_DIR) 为实际的 object_output 路径
            let replaced_opt = opt.replace("$(TARGET_OBJECT_DIR)", &project_info.object_output);
            pre_link_flags.push(replaced_opt);
        }
        // 添加链接库目录（-L选项）
        for lib_dir in &project_info.linker_lib_dirs {
            pre_link_flags.push(lib_dir.clone());
        }

        // 修改链接规则，将库标志放在目标文件之后
        ninja_content.push_str("rule link\n");
        ninja_content.push_str(&format!(
            "  command = {} $pre_flags $in $lib_flags -o $out\n",
            linker
        ));
        ninja_content.push_str("\n");

        // 构建隐式依赖列表 (| 后面)
        // 包含特殊对象文件和已解析的库文件
        let mut implicit_deps = Vec::new();
        implicit_deps.extend(special_output_files.iter().cloned());
        implicit_deps.extend(resolved_lib_dependencies.iter().cloned());

        let implicit_deps_str = if implicit_deps.is_empty() {
            String::new()
        } else {
            format!(" | {}", implicit_deps.join(" "))
        };

        // 生成可执行文件构建规则
        ninja_content.push_str(&format!(
            "build {}: link {}{}\n",
            target_name,
            regular_obj_files.join(" "),
            implicit_deps_str
        ));

        // 添加前导链接标志
        if !pre_link_flags.is_empty() {
            ninja_content.push_str(&format!("  pre_flags = {}\n", pre_link_flags.join(" ")));
        }

        // 添加库链接标志，放在目标文件之后
        if !lib_flags.is_empty() {
            ninja_content.push_str(&format!("  lib_flags = {}\n", lib_flags.join(" ")));
        }
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
    _project_dir: &Path,
    ninja_path: Option<&str>,
) -> String {
    debug_println!("[DEBUG generator] Starting to generate build script...");

    let mut script_content = String::new();

    // 1. 添加工具链路径到PATH环境变量
    let toolchain_bin = format!(r"{}\bin", toolchain.get_base_path());
    script_content.push_str("@echo off\n");
    script_content.push_str("rem Generated by cbp2clangd\n");
    script_content.push_str("\n");
    script_content.push_str("cd /d \"%~dp0\"\n\n");
    script_content.push_str("rem Set toolchain path\n");
    script_content.push_str(&format!("set PATH={};%PATH%\n", toolchain_bin));

    script_content.push_str("\n");

    // 2. 添加预构建命令
    if !project_info.prebuild_commands.is_empty() {
        script_content.push_str("rem Prebuild commands\n");
        for cmd in &project_info.prebuild_commands {
            script_content.push_str("pushd %~dp0\n");
            // 替换 $(PROJECT_NAME) 变量
            let processed_cmd = cmd.replace("$(PROJECT_NAME)", &project_info.project_name);
            script_content.push_str(&format!("call {}\n", processed_cmd));
            script_content.push_str("popd\n");
        }
        script_content.push_str("\n");
    }

    // 3. 添加ninja构建命令
    script_content.push_str("rem Build project with ninja\n");
    if let Some(ninja_path) = ninja_path {
        script_content.push_str(&format!("{} -f build.ninja\n", ninja_path));
    } else {
        script_content.push_str("ninja -f build.ninja\n");
    }
    script_content.push_str("if %errorlevel% neq 0 exit /b %errorlevel%\n");
    script_content.push_str("\n");

    // 4. 添加后构建命令
    if !project_info.postbuild_commands.is_empty() {
        script_content.push_str("rem Postbuild commands\n");
        for cmd in &project_info.postbuild_commands {
            script_content.push_str("pushd %~dp0\n");
            // 替换 $(PROJECT_NAME) 变量
            let processed_cmd = cmd.replace("$(PROJECT_NAME)", &project_info.project_name);
            script_content.push_str(&format!("call {}\n", processed_cmd));
            script_content.push_str("popd\n");
        }
        script_content.push_str("\n");
    }

    // 5. 添加完成信息
    script_content.push_str("rem Build completed successfully\n");
    script_content.push_str("echo Build completed successfully\n");
    script_content.push_str("\n");

    debug_println!("[DEBUG generator] Successfully generated build script content");
    script_content
}
