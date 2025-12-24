use crate::config::ToolchainConfig;
use crate::debug_println;
use crate::models::CompileCommand;
use crate::parser::ProjectInfo;
use crate::utils::get_short_path;
use std::path::{Component, Path, PathBuf};

/// 辅助函数：将Path转换为Windows风格的字符串路径（使用反斜杠作为分隔符）
fn normalize_path(path: &Path) -> String {
    let path_str = path.to_string_lossy().into_owned();
    // 确保在所有平台上都使用Windows风格的路径分隔符
    path_str.replace("/", "\\")
}

/// 新增辅助函数：直接标准化字符串类型的路径
fn normalize_str(s: &str) -> String {
    s.replace("/", "\\")
}

/// 新增核心函数：清洗构建参数（Flags）
/// 这是一个系统性的解决方案，用于处理 "-Ipath/to", "-Lpath/to", "path/to/file.a" 等各种情况
fn sanitize_flag(flag: &str) -> String {
    // 简单直接的策略：在 Windows 环境生成场景下，将所有正斜杠替换为反斜杠
    // 这对于 GCC/Clang 的路径参数（-I, -L, -o, 纯文件名）都是安全的
    // 同时也统一了视觉风格
    flag.replace("/", "\\")
}

/// 辅助函数：逻辑上解析绝对路径（不依赖文件系统存在性，仅处理路径组件）
/// 用于解决 project_dir + ../../file.c 的路径计算
fn get_clean_absolute_path(base: &Path, rel: &Path) -> PathBuf {
    let mut result = base.to_path_buf();
    for component in rel.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(c) => {
                result.push(c);
            }
            Component::RootDir => {
                // 如果遇到根目录（如Linux的 / 或 Windows 的 \），重置路径
                // 注意：在Windows上处理盘符比较复杂，这里简化处理，假设是相对路径
                result = PathBuf::from(component.as_os_str());
            }
            Component::Prefix(prefix) => {
                 // Windows 盘符，重置路径
                 result = PathBuf::from(prefix.as_os_str());
            }
            Component::CurDir => {}
        }
    }
    result
}

/// 辅助函数：计算一组路径的共同祖先目录
fn find_common_ancestor(paths: &[PathBuf]) -> PathBuf {
    if paths.is_empty() {
        return PathBuf::from(".");
    }
    
    // 以第一个路径作为初始祖先
    let mut ancestor = paths[0].parent().unwrap_or(Path::new("")).to_path_buf();
    
    for path in paths.iter().skip(1) {
        // 逐级向上回退，直到 ancestor 是 path 的前缀
        while !path.starts_with(&ancestor) {
            if !ancestor.pop() {
                // 如果回退到空，说明没有共同祖先（例如跨盘符），返回空或根
                // 在这种极端情况下，通常意味着无法保持目录结构，返回当前目录作为fallback
                return PathBuf::from("."); 
            }
        }
    }
    ancestor
}

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

/// 辅助函数：计算 target 相对于 base 的相对路径
fn compute_relative_path(target: &Path, base: &Path) -> Option<PathBuf> {
    let target_canon = target.canonicalize().ok()?;
    let base_canon = base.canonicalize().ok()?;

    let mut ita = target_canon.components();
    let mut itb = base_canon.components();
    let mut comps: Vec<Component> = vec![];

    loop {
        match (ita.next(), itb.next()) {
            (None, None) => break,
            (Some(a), None) => {
                comps.push(a);
                comps.extend(ita);
                break;
            }
            (None, _) => comps.push(Component::ParentDir),
            (Some(a), Some(b)) if a == b => continue, // 路径相同，继续
            (Some(a), Some(_)) => {
                // 路径出现分歧，base 需要回退 (ParentDir)，target 需要继续
                comps.push(Component::ParentDir);
                for _ in itb {
                    comps.push(Component::ParentDir);
                }
                comps.push(a);
                comps.extend(ita);
                break;
            }
        }
    }

    Some(comps.iter().map(|c| c.as_os_str()).collect())
}

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

    // 定义一个闭包来统一处理“找到路径后”的逻辑
    let finalize_path = |found_path: PathBuf| -> String {
        if let Some(rel) = compute_relative_path(&found_path, root_dir) {
            normalize_path(&rel)
        } else {
            normalize_path(&found_path)
        }
    };

    // 2. 如果不是 flag (-l)，直接检查文件路径
    if !is_flag {
        let p = Path::new(lib);
        let full_p = if p.is_absolute() {
            p.to_path_buf()
        } else {
            root_dir.join(p)
        };
        
        if full_p.exists() {
            return Some(finalize_path(full_p));
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
                return Some(finalize_path(full_path));
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

    // 使用工具链中的编译器路径
    let compiler_path = toolchain.compiler_path();
    let compiler_exists = Path::new(&compiler_path).exists();
    let compiler = if compiler_exists {
        match get_short_path(&compiler_path) {
            Ok(short_path) => short_path,
            Err(e) => {
                println!("[WARNING generator] Failed to get short path for compiler: {}. Using original path.", e);
                compiler_path.clone()
            }
        }
    } else {
        println!("[WARNING generator] Compiler path {} does not exist. Using placeholder.", compiler_path);
        "riscv32-elf-gcc".to_string()
    };

    // 获取链接器路径
    let linker_path = toolchain.linker_path(&project_info.linker_type);
    let linker_exists = Path::new(&linker_path).exists();
    let linker = if linker_exists {
        match get_short_path(&linker_path) {
            Ok(short_path) => short_path,
            Err(e) => {
                println!("[WARNING generator] Failed to get short path for linker: {}. Using original path.", e);
                linker_path.clone()
            }
        }
    } else {
        println!("[WARNING generator] Linker path {} does not exist. Using placeholder.", linker_path);
        if project_info.linker_type == "ld" {
            "riscv32-elf-ld".to_string()
        } else {
            "riscv32-elf-gcc".to_string()
        }
    };

    // 提前计算常用的标准化路径，避免重复计算
    let clean_obj_dir = normalize_path(Path::new(&project_info.object_output));

    // 构建基础编译器标志
    let mut base_flags: Vec<String> = Vec::new();
    for flag in &project_info.global_cflags {
        base_flags.push(sanitize_flag(flag)); // 确保全局CFLAGS里的路径也被转换
    }
    for include in &project_info.include_dirs {
        // -I 选项
        let clean_path = normalize_path(Path::new(include));
        base_flags.push(format!("{}", clean_path));
    }

    // 规则部分
    let mut ninja_content = String::new();
    ninja_content.push_str("# Generated by cbp2clangd\n");
    ninja_content.push_str("\n");
    
    // Rule: CC
    ninja_content.push_str("rule cc\n");
    ninja_content.push_str(&format!(
        "  command = {} $flags -MMD -MF $out.d -c $in -o $out\n",
        compiler
    ));
    ninja_content.push_str("  depfile = $out.d\n");
    ninja_content.push_str("  deps = gcc\n");
    ninja_content.push_str("\n");

    // === 新增逻辑：计算所有源文件的共同祖先目录，以保持目录结构 ===
    // 1. 获取所有源文件的逻辑绝对路径
    let abs_source_paths: Vec<PathBuf> = project_info.source_files.iter()
        .map(|src| get_clean_absolute_path(project_dir, Path::new(src)))
        .collect();

    // 2. 找到共同祖先目录
    let common_ancestor = find_common_ancestor(&abs_source_paths);
    debug_println!("[DEBUG generator] Common source ancestor: {}", common_ancestor.display());

    // 构建对象文件列表
    let mut regular_obj_files = Vec::new();
    // 保存源文件路径和对应的对象文件路径的映射
    let mut src_to_obj_map = Vec::new();

    // 处理普通源文件
    // 同时遍历 原始相对路径 和 计算出的绝对路径
    for (src, abs_path) in project_info.source_files.iter().zip(abs_source_paths.iter()) {
        let src_path = Path::new(src);

        // 3. 计算相对于共同祖先的路径
        // 如果 strip_prefix 失败（例如跨盘符），回退到使用文件名
        let relative_structure = abs_path.strip_prefix(&common_ancestor)
            .unwrap_or_else(|_| match src_path.file_name() {
                Some(name) => Path::new(name),
                None => src_path,
            });

        // 4. 构建最终的对象文件路径：object_output + 相对结构 + .o
        let obj_path_buf = Path::new(&project_info.object_output)
            .join(relative_structure)
            .with_extension("o");

        let obj_name = normalize_path(&obj_path_buf);
        let clean_src = normalize_path(src_path);
        
        regular_obj_files.push(obj_name.clone());
        src_to_obj_map.push((clean_src, obj_name));
    }
    // === 结束新增逻辑 ===

    // 处理特殊文件（只编译，不链接）
    let mut special_output_files = Vec::new();
    for special_file in &project_info.special_files {
        // 解析构建命令中的目标文件名
        let mut processed_cmd = special_file.build_command.clone();

        // 路径标准化处理
        let clean_file_path = normalize_path(Path::new(&special_file.filename));
        let clean_includes = project_info.include_dirs.iter()
            .map(|p| normalize_path(Path::new(p)))
            .collect::<Vec<_>>()
            .join(" ");

        // 替换变量
        processed_cmd = processed_cmd.replace("$compiler", &compiler);
        processed_cmd = processed_cmd.replace("$options", &base_flags.join(" "));
        processed_cmd = processed_cmd.replace("$includes", &clean_includes);
        processed_cmd = processed_cmd.replace("$file", &clean_file_path);
        processed_cmd = processed_cmd.replace("$(TARGET_OBJECT_DIR)", &clean_obj_dir);
        processed_cmd = processed_cmd.replace("$(TARGET_OUTPUT_DIR)", &clean_obj_dir);

        // 提取输出文件名
        let output_file = if let Some(output_pos) = processed_cmd.find("-o ") {
            let rest = &processed_cmd[output_pos + 3..];
            let raw_out = if let Some(space_pos) = rest.find(' ') {
                &rest[..space_pos]
            } else {
                rest
            };
            normalize_path(Path::new(raw_out))
        } else {
            // 对特殊文件也应用类似的逻辑，尝试保持结构，但因为它是自定义命令，
            // 通常由用户指定输出位置。这里只做简单的 fallback
            let abs_path = get_clean_absolute_path(project_dir, Path::new(&special_file.filename));
            let relative_structure = abs_path.strip_prefix(&common_ancestor)
                .unwrap_or_else(|_| Path::new(&special_file.filename));
            
            let full_path = Path::new(&project_info.object_output)
                .join(relative_structure)
                .with_extension("o");
                
            normalize_path(&full_path)
        };

        special_output_files.push(output_file.clone());

        let rule_name = format!(
            "special_{}",
            special_file
                .filename
                .replace(".", "_")
                .replace("/", "_")
                .replace("\\", "_")
                .replace(":", "_")
        );
        ninja_content.push_str(&format!("rule {}\n", rule_name));
        ninja_content.push_str(&format!("  command = {}\n", processed_cmd));
        ninja_content.push_str("\n");

        ninja_content.push_str(&format!(
            "build {}: {} {}\n",
            output_file, rule_name, clean_file_path
        ));
        ninja_content.push_str("\n");
    }

    // 构建部分 - 普通源文件
    for (src, obj) in src_to_obj_map {
        ninja_content.push_str(&format!("build {}: cc {}\n", obj, src));
        ninja_content.push_str(&format!("  flags = {}\n", base_flags.join(" ")));
        ninja_content.push_str("\n");
    }

    // 链接目标
    let mut target_name = normalize_path(Path::new(&project_info.output));

    // 检查是否为静态库目标（.a结尾）
    let is_static_lib = target_name.ends_with(".a");

    // 处理静态库文件名
    if is_static_lib {
        let target_path = Path::new(&target_name);
        if let Some(file_name) = target_path.file_name() {
            let file_name_str = file_name.to_string_lossy().to_string();
            if !file_name_str.starts_with("lib") {
                let dir = target_path.parent().unwrap_or_else(|| Path::new("."));
                let stem = file_name_str.strip_suffix(".a").unwrap_or(&file_name_str);
                let new_file_name = format!("lib{}.a", stem);
                target_name = normalize_path(&dir.join(new_file_name));
            }
        }
    }

    // 生成主目标的构建规则
    if is_static_lib {
        // 静态库目标
        let ar_path = toolchain.ar_path();
        let ar_exists = Path::new(&ar_path).exists();
        let ar = if ar_exists {
            match get_short_path(&ar_path) {
                Ok(short_path) => short_path,
                Err(e) => {
                    println!("[WARNING generator] Failed to get short path for ar: {}. Using original.", e);
                    ar_path.clone()
                }
            }
        } else {
            println!("[WARNING generator] Ar path {} does not exist. Using placeholder.", ar_path);
            "riscv32-elf-ar".to_string()
        };

        ninja_content.push_str("rule ar\n");
        ninja_content.push_str(&format!(
            "  command = cmd /c (if exist \"$out\" del /q \"$out\") & {} crs $out $in\n",
            ar
        ));
        ninja_content.push_str("\n");

        let deps_str = if special_output_files.is_empty() {
             String::new()
        } else {
             format!(" | {}", special_output_files.join(" "))
        };

        ninja_content.push_str(&format!(
            "build {}: ar {}{}\n",
            target_name,
            regular_obj_files.join(" "),
            deps_str
        ));

    } else {
        // 可执行文件目标
        let mut pre_link_flags: Vec<String> = Vec::new();
        let mut lib_flags: Vec<String> = Vec::new();
        
        // 1. 解析库依赖
        let mut resolved_lib_dependencies = Vec::new();

        debug_println!("[DEBUG generator] Resolving library dependencies...");
        
        for lib in &project_info.linker_libs {
            // 在这里应用 sanitize_flag
            // 这样无论是 "-lmath", "libs/libmath.a", 还是 "../libs/libfoo.a" 
            // 都会变成 Windows 风格 (../libs/libfoo.a -> ..\libs\libfoo.a)
            lib_flags.push(sanitize_flag(lib)); 

            // 依赖解析逻辑（用于 ninja 的 implicit deps）
            if let Some(resolved_path) = resolve_library_path(lib, &project_info.linker_lib_dirs, project_dir) {
                debug_println!("[DEBUG generator] Resolved library {} to {}", lib, resolved_path);
                resolved_lib_dependencies.push(resolved_path);
            } else {
                debug_println!("[DEBUG generator] Could not resolve library path for {}", lib);
            }
        }

        // 添加链接器选项
        for opt in &project_info.linker_options {
            let replaced_opt = opt.replace("$(TARGET_OBJECT_DIR)", &clean_obj_dir);
            // Linker options 可能包含 -Map=output/path.map 之类的，需要转换路径分隔符
            pre_link_flags.push(sanitize_flag(&replaced_opt));
        }
        // 添加链接库目录
        for lib_dir in &project_info.linker_lib_dirs {
            // 统一处理 -L 标志
            if lib_dir.starts_with("-L") {
                let path_part = &lib_dir[2..];
                // normalize_str 替换斜杠
                let clean_part = normalize_str(path_part);
                pre_link_flags.push(format!("-L{}", clean_part));
            } else {
                pre_link_flags.push(normalize_str(lib_dir));
            }
        }

        ninja_content.push_str("rule link\n");
        ninja_content.push_str(&format!(
            "  command = {} $pre_flags $in $lib_flags -o $out\n",
            linker
        ));
        ninja_content.push_str("\n");

        let mut implicit_deps = Vec::new();
        implicit_deps.extend(special_output_files.iter().cloned());
        implicit_deps.extend(resolved_lib_dependencies.iter().cloned());

        let implicit_deps_str = if implicit_deps.is_empty() {
            String::new()
        } else {
            format!(" | {}", implicit_deps.join(" "))
        };

        ninja_content.push_str(&format!(
            "build {}: link {}{}\n",
            target_name,
            regular_obj_files.join(" "),
            implicit_deps_str
        ));

        if !pre_link_flags.is_empty() {
            ninja_content.push_str(&format!("  pre_flags = {}\n", pre_link_flags.join(" ")));
        }

        if !lib_flags.is_empty() {
            ninja_content.push_str(&format!("  lib_flags = {}\n", lib_flags.join(" ")));
        }
    }
    ninja_content.push_str("\n");

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
