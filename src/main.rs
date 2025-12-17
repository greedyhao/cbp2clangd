use std::env;
use std::fs;

use cbp2clangd::{debug_println, generate_build_script, generate_clangd_config, generate_compile_commands, generate_ninja_build, parse_args, parse_cbp_file, set_debug_mode, ToolchainConfig};

// 项目版本信息
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    debug_println!("[DEBUG] Parsing command line arguments...");
    let args = parse_args()?;

    // 设置调试模式
    set_debug_mode(args.debug);

    debug_println!("[DEBUG] Starting cbp2clangd v{}", VERSION);
    debug_println!(
        "[DEBUG main] 调试模式已{}",
        if args.debug { "启用" } else { "禁用" }
    );

    // 如果请求显示版本信息，则打印版本并退出
    if args.show_version {
        println!("cbp2clangd v{}", VERSION);
        return Ok(());
    }

    // 读取并解析项目文件
    debug_println!("[DEBUG] Reading project file...");
    let cbp_path = args.cbp_path.as_ref().unwrap();
    let output_dir = args.output_dir.as_ref().unwrap();

    debug_println!("[DEBUG] CBP path: {}", cbp_path.display());
    debug_println!("[DEBUG] Output dir: {}", output_dir.display());
    debug_println!("[DEBUG] Linker type: {}", args.linker_type);

    // 测试模式：使用内置的XML内容
    let xml_content = if args.test_mode {
        // 内置的测试XML内容，包含动态库输出和Build/Target/Linker/Add directory
        String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="chatbot" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/chatbot.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
                <Linker>
                    <Add library="m" />
                    <Add directory="../../platform/libs/net" />
                </Linker>
            </Target>
        </Build>
        <Compiler>
            <Add option="-Wall" />
            <Add option="-g" />
        </Compiler>
        <Linker>
            <Add option="-Wl,--gc-sections" />
        </Linker>
        <Unit filename="src/chatbot.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#)
    } else {
        // 正常模式：读取文件内容
        debug_println!("[DEBUG] Checking if CBP file exists...");
        if !cbp_path.exists() {
            return Err(format!("CBP file not found: {}", cbp_path.display()).into());
        }

        debug_println!("[DEBUG] Reading CBP file content...");
        fs::read_to_string(cbp_path)?
    };

    debug_println!("[DEBUG] Parsing CBP file...");
    let mut project_info = parse_cbp_file(&xml_content)?;

    // 使用命令行参数中的linker_type覆盖解析结果
    project_info.linker_type = args.linker_type;

    // 确定工具链配置
    debug_println!(
        "[DEBUG] Determining toolchain configuration for compiler: {}",
        project_info.compiler_id
    );
    let toolchain = ToolchainConfig::from_compiler_id(&project_info.compiler_id)
        .unwrap_or_else(|| {
            eprintln!(
                "Warning: Unknown compiler '{}', falling back to v2",
                project_info.compiler_id
            );
            ToolchainConfig::from_compiler_id("riscv32-v2").unwrap()
        });
    debug_println!("[DEBUG] Toolchain config created successfully");

    // 检查编译器是否可用
    if !toolchain.is_compiler_available() {
        eprintln!("Error: Compiler not found at {}", toolchain.compiler_path());
        eprintln!(
            "Suggestion: The compiler path may be incorrect or the toolchain is not installed."
        );
        eprintln!("You can try:");
        eprintln!("  1. Install the RV32-Toolchain in the default location");
        eprintln!("  2. Use a custom toolchain path (to be implemented in future versions)");

        // 为了让程序能够继续运行，即使编译器不可用，我们仍然生成配置文件
        // 但会使用一个合理的默认编译器名称而不是路径
        eprintln!(
            "\nNote: Continuing with configuration generation using a placeholder compiler path..."
        );
    }

    // 生成编译命令列表
    debug_println!("[DEBUG] Generating project directory path...");
    let project_dir = if args.test_mode {
        // 测试模式：直接使用当前目录
        std::env::current_dir()?
    } else {
        // 正常模式：获取cbp_path的父目录的规范路径
        cbp_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .canonicalize()?
    };
    debug_println!("[DEBUG] Project directory: {}", project_dir.display());

    // 生成编译命令列表
    debug_println!("[DEBUG] Generating compile commands...");
    let compile_commands = 
        generate_compile_commands(&project_info, &project_dir, &toolchain);
    debug_println!(
        "[DEBUG] Compile commands generated: {}",
        compile_commands.len()
    );

    // 生成 compile_commands.json
    debug_println!("[DEBUG] Preparing compile_commands.json path...");

    // 首先规范化输出目录路径
    debug_println!(
        "[DEBUG] Normalizing output directory path: {}",
        output_dir.display()
    );
    let normalized_output_dir = if !output_dir.is_absolute() {
        project_dir.join(output_dir)
    } else {
        output_dir.clone()
    };
    let normalized_output_dir = normalized_output_dir.canonicalize()?;
    debug_println!(
        "[DEBUG] Normalized output directory: {}",
        normalized_output_dir.display()
    );

    // 确保输出目录存在
    debug_println!("[DEBUG] Ensuring output directory exists...");
    std::fs::create_dir_all(&normalized_output_dir)?;
    debug_println!("[DEBUG] Output directory ensured");

    // 使用规范化后的目录创建compile_commands.json路径
    let compile_commands_path = normalized_output_dir.join("compile_commands.json");
    debug_println!(
        "[DEBUG] Final compile_commands.json path: {}",
        compile_commands_path.display()
    );
    debug_println!(
        "[DEBUG] After canonicalize: {}",
        compile_commands_path.display()
    );

    debug_println!("[DEBUG] Creating parent directory if needed...");
    let parent_dir = compile_commands_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""));
    debug_println!("[DEBUG] Parent directory: {}", parent_dir.display());
    fs::create_dir_all(parent_dir)?;

    debug_println!("[DEBUG] Serializing compile commands to JSON...");
    let json_content = serde_json::to_string_pretty(&compile_commands)?;

    debug_println!(
        "[DEBUG] Writing compile_commands.json to: {}",
        compile_commands_path.display()
    );
    fs::write(&compile_commands_path, json_content)?;
    println!("Generated {}", compile_commands_path.display());

    // 生成 .clangd 配置文件
    debug_println!("[DEBUG] Generating .clangd config content...");
    let clangd_content = generate_clangd_config(&project_info, &toolchain)?;

    debug_println!("[DEBUG] Preparing .clangd file path...");

    // 使用已经规范化的输出目录创建.clangd路径
    let clangd_path = normalized_output_dir.join(".clangd");
    debug_println!("[DEBUG] Final .clangd path: {}", clangd_path.display());

    debug_println!(
        "[DEBUG] Writing .clangd config to: {}",
        clangd_path.display()
    );
    fs::write(&clangd_path, clangd_content)?;
    println!("Generated {}", clangd_path.display());

    // 生成 ninja 构建文件
    debug_println!("[DEBUG] Generating ninja build content...");
    let ninja_content = generate_ninja_build(&project_info, &project_dir, &toolchain)?;

    debug_println!("[DEBUG] Preparing ninja build file path...");
    // 根据需求，build.ninja 必须放在 cbp 工程同一路径
    let ninja_path = project_dir.join("build.ninja");
    debug_println!(
        "[DEBUG] Final ninja build file path: {}",
        ninja_path.display()
    );

    debug_println!(
        "[DEBUG] Writing ninja build file to: {}",
        ninja_path.display()
    );
    fs::write(&ninja_path, ninja_content)?;
    println!("Generated {}", ninja_path.display());

    // 生成构建脚本文件
    debug_println!("[DEBUG] Generating build script...");
    let build_script_content = 
        generate_build_script(&project_info, &toolchain, &project_dir, args.ninja_path.as_deref());
    let build_script_path = project_dir.join("build.bat");
    debug_println!(
        "[DEBUG] Writing build script to: {}",
        build_script_path.display()
    );
    fs::write(&build_script_path, build_script_content)?;
    println!("Generated {}", build_script_path.display());

    debug_println!("[DEBUG] Program completed successfully");
    Ok(())
}
