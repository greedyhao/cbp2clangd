use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use cbp2clangd::{
    ToolchainConfig, debug_println, generate_build_script, generate_compile_commands,
    generate_ninja_build, parse_args, parse_cbp_file, set_debug_mode,
    // 引入两个生成函数
    generate_clangd_config, generate_clangd_fragment,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    debug_println!("[DEBUG] Parsing command line arguments...");
    let args = parse_args()?;
    set_debug_mode(args.debug);

    if args.show_version {
        println!("cbp2clangd v{}", VERSION);
        return Ok(());
    }

    let cbp_path = args.cbp_path.as_ref().unwrap();
    // output_dir 在 cli.rs 中已经处理过，这里直接获取
    let cli_output_dir = args.output_dir.as_ref().unwrap();

    // 确保 workspace_root 是绝对路径 (用于 .clangd 计算相对路径)
    let workspace_root = if cli_output_dir.is_absolute() {
        if cli_output_dir.exists() {
            cli_output_dir.canonicalize()?
        } else {
            cli_output_dir.clone()
        }
    } else {
        // 理论上 cli.rs 已经处理了相对路径转绝对，但这里防御性处理一下
        fs::create_dir_all(cli_output_dir)?;
        cli_output_dir.canonicalize()?
    };

    debug_println!("[DEBUG] Workspace Root: {}", workspace_root.display());

    // 读取 CBP
    let xml_content = if args.test_mode {
        // ... (测试内容)
         String::from(r#"<?xml version="1.0" encoding="UTF-8"?><CodeBlocks_project_file><Project><Option title="chatbot"/><Build><Target title="Debug"><Option output="Output/bin/chatbot.elf" object_output="Output/obj/Debug"/><Linker><Add library="m"/></Linker></Target></Build><Compiler><Add option="-Wall"/></Compiler><Unit filename="src/chatbot.c"><Option compile="1"/></Unit></Project></CodeBlocks_project_file>"#)
    } else {
        if !cbp_path.exists() {
            return Err(format!("CBP file not found: {}", cbp_path.display()).into());
        }
        fs::read_to_string(cbp_path)?
    };

    let mut project_info = parse_cbp_file(&xml_content)?;
    project_info.linker_type = args.linker_type;

    let toolchain = ToolchainConfig::from_compiler_id(&project_info.compiler_id).unwrap_or_else(|| {
        eprintln!("Warning: Unknown compiler, falling back to v2");
        ToolchainConfig::from_compiler_id("riscv32-v2").unwrap()
    });

    // 项目根目录
    let project_dir = if args.test_mode {
        std::env::current_dir()?
    } else {
        cbp_path.parent().unwrap_or(Path::new(".")).canonicalize()?
    };

    // 1. 处理 Object Output (存放 CDB 和 bat)
    let raw_obj_out = &project_info.object_output;
    let abs_object_output = project_dir.join(raw_obj_out);
    fs::create_dir_all(&abs_object_output)?;
    let abs_object_output = abs_object_output.canonicalize()?;
    
    debug_println!("[DEBUG] Object Output: {}", abs_object_output.display());

    // 2. 生成 compile_commands.json
    let compile_commands = generate_compile_commands(&project_info, &project_dir, &toolchain);
    let cdb_path = abs_object_output.join("compile_commands.json");
    fs::write(&cdb_path, serde_json::to_string_pretty(&compile_commands)?)?;
    println!("Generated {}", cdb_path.display());

    // 3. 生成 build.ninja (放在 Project Dir)
    let ninja_content = generate_ninja_build(&project_info, &project_dir, &toolchain)?;
    let ninja_path = project_dir.join("build.ninja");
    fs::write(&ninja_path, ninja_content)?;
    println!("Generated {}", ninja_path.display());

    // 生成构建脚本文件
    debug_println!("[DEBUG] Generating build script...");
    let build_script_content = generate_build_script(
        &project_info,
        &toolchain,
        &project_dir,
        args.ninja_path.as_deref(),
    );
    let build_script_path = project_dir.join("build.bat");
    debug_println!(
        "[DEBUG] Writing build script to: {}",
        build_script_path.display()
    );
    fs::write(&build_script_path, build_script_content)?;
    println!("Generated {}", build_script_path.display());

    // 5. 处理 .clangd (在 Workspace Root)
    let clangd_path = workspace_root.join(".clangd");

    // A. 生成公共头部 (Base Config)
    let base_config = generate_clangd_config(&project_info, &toolchain)?;
    
    // B. 生成项目专属片段 (Fragment)
    let (current_path_match, fragment_content) = generate_clangd_fragment(
        &project_info,
        &project_dir,
        &workspace_root,
        &abs_object_output
    )?;

    // C. 读取并合并
    let existing_content = if clangd_path.exists() {
        fs::read_to_string(&clangd_path)?
    } else {
        String::new()
    };

    let mut final_parts = Vec::new();

    if existing_content.trim().is_empty() {
        // 新文件：Header + Fragment
        final_parts.push(base_config);
    } else {
        // 旧文件：分割处理
        let parts: Vec<&str> = existing_content.split("\n---").collect();

        // 策略：始终更新头部为当前项目的生成配置 (或者保留旧的，这取决于是否希望"最后一次运行"决定公共配置)
        // 鉴于用户说"公共部分需要靠 generate_clangd_config 生成"，我们这里选择用新生成的 Base Config 替换旧文件的头部
        // 这样可以确保配置是最新的。
        debug_println!("[DEBUG] Updating common config header...");
        final_parts.push(base_config);

        // 处理后续片段
        for part in parts.iter().skip(1) {
            let trimmed_part = part.trim();
            // 如果片段的 PathMatch 与当前生成的不同，则保留；如果相同，则丢弃(稍后追加新的)
            if !trimmed_part.contains(&format!("PathMatch: {}", current_path_match)) {
                final_parts.push(trimmed_part.to_string());
            } else {
                debug_println!("[DEBUG] Replacing existing config for {}", current_path_match);
            }
        }
    }

    // 追加当前片段
    final_parts.push(fragment_content);

    // 写入
    fs::write(&clangd_path, final_parts.join("\n\n---\n"))?;
    println!("Updated {} (Merged config for {})", clangd_path.display(), current_path_match);

    Ok(())
}