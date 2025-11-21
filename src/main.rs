mod cli;
mod config;
mod generator;
mod models;
mod parser;
mod utils;

use std::fs;

// 项目版本信息
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    let args = cli::parse_args()?;
    
    // 如果请求显示版本信息，则打印版本并退出
    if args.show_version {
        println!("cbp2clangd v{}", VERSION);
        return Ok(());
    }

    // 读取并解析项目文件
    let cbp_path = args.cbp_path.as_ref().unwrap();
    let output_dir = args.output_dir.as_ref().unwrap();
    
    let xml_content = fs::read_to_string(cbp_path)?;
    let project_info = parser::parse_cbp_file(&xml_content)?;

    // 确定工具链配置
    let toolchain = config::ToolchainConfig::from_compiler_id(&project_info.compiler_id)
        .unwrap_or_else(|| {
            eprintln!(
                "Warning: Unknown compiler '{}', falling back to v2",
                project_info.compiler_id
            );
            config::ToolchainConfig::from_compiler_id("riscv32-v2").unwrap()
        });

    // 生成编译命令列表
    let project_dir = cbp_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .canonicalize()?;
    let compile_commands = 
        generator::generate_compile_commands(&project_info, &project_dir, &toolchain);

    // 生成 compile_commands.json
    let compile_commands_path = output_dir.join("compile_commands.json");
    fs::create_dir_all(output_dir)?;
    let json_content = serde_json::to_string_pretty(&compile_commands)?;
    
    // 尝试获取短路径来写入文件
    let display_path = match utils::get_short_path(&compile_commands_path) {
        Ok(short_path) => short_path,
        Err(e) => {
            eprintln!("Warning: Failed to get short path for output file: {}. Using original path.", e);
            compile_commands_path.to_string_lossy().to_string()
        }
    };
    
    fs::write(&compile_commands_path, json_content)?;
    println!("Generated {}", display_path);

    // 生成 .clangd 配置文件
    let clangd_content = generator::generate_clangd_config(&project_info, &toolchain)?;
    let clangd_path = output_dir.join(".clangd");
    
    // 尝试获取短路径来写入文件
    let clangd_display_path = match utils::get_short_path(&clangd_path) {
        Ok(short_path) => short_path,
        Err(e) => {
            eprintln!("Warning: Failed to get short path for output file: {}. Using original path.", e);
            clangd_path.to_string_lossy().to_string()
        }
    };
    
    fs::write(&clangd_path, clangd_content)?;
    println!("Generated {}", clangd_display_path);

    Ok(())
}
