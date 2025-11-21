use crate::config::ToolchainConfig;
use crate::models::CompileCommand;
use crate::parser::ProjectInfo;
use crate::utils::get_short_path;
use std::path::Path;

/// 生成clangd配置文件内容
pub fn generate_clangd_config(
    project_info: &ProjectInfo,
    toolchain: &ToolchainConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    let includes = toolchain
        .include_paths()
        .iter()
        .map(|p| format!("-I{}", p))
        .collect::<Vec<_>>();

    // 构建Add部分
    let mut add_flags = vec!["-xc", "-target", "riscv32-unknown-elf"];

    // 添加include路径
    for inc in &includes {
        add_flags.push(&inc[..]);
    }

    if project_info.march_info.has_custom_extension {
        if let Some(base_march) = &project_info.march_info.base_march {
            add_flags.push(base_march.as_str());
        }
    }

    // 构建Remove部分
    let mut remove_flags = Vec::new();
    // 如果有-march指令，添加到Remove部分
    if !project_info.march_info.full_march.is_empty() {
        remove_flags.push(&project_info.march_info.full_march[..]);
    }
    remove_flags.push("-mjump-tables-in-text");

    // 注意：.clangd 是 YAML，但 clangd 接受这种简写格式
    let mut content = format!("CompileFlags:\n  Add:\n");

    // 添加Add部分
    for flag in add_flags {
        content.push_str(&format!("    - {}\n", flag.replace('\\', "\\\\")));
    }

    // 如果有Remove部分，添加
    if !remove_flags.is_empty() {
        content.push_str("  Remove:\n");
        for flag in remove_flags {
            content.push_str(&format!("    - {}\n", flag));
        }
    }

    Ok(content)
}

/// 生成编译命令列表
pub fn generate_compile_commands(
    project_info: &crate::parser::ProjectInfo,
    project_dir: &Path,
    toolchain: &ToolchainConfig,
) -> Vec<CompileCommand> {
    // 使用toolchain中的编译器路径
    let compiler = toolchain.compiler_path();
    // 尝试获取编译器的短路径名
    let compiler = match get_short_path(&compiler) {
        Ok(short_path) => short_path,
        Err(e) => {
            eprintln!("Warning: Failed to get short path for compiler: {}. Using original path.", e);
            // 如果失败，使用长文件名路径
            format!("\\\\?\\\\{}", compiler)
        }
    };

    let base_flags: Vec<String> = project_info
        .global_cflags
        .iter()
        .cloned()
        .chain(project_info.include_dirs.iter().cloned())
        .collect();

    let mut compile_commands = Vec::new();
    for src in &project_info.source_files {
        // 尝试获取源文件的短路径名
        let src_path = match get_short_path(src) {
            Ok(short_path) => short_path,
            Err(e) => {
                eprintln!("Warning: Failed to get short path for source file {}: {}. Using original path.", src, e);
                src.clone()
            }
        };

        let mut cmd = vec![&compiler[..], "-c"];
        cmd.extend(base_flags.iter().map(|s| s.as_str()));
        cmd.push(&src_path);

        compile_commands.push(CompileCommand {
            directory: project_dir.to_string_lossy().into_owned(),
            command: cmd.join(" "),
            file: src.clone(), // 保留原始文件名用于引用
        });
    }

    compile_commands
}
