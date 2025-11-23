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
