use std::env;
use std::path::{Path, PathBuf};

use crate::parser::parse_cbp_file;

/// 转换命令参数（原有的 CBP 转换功能）
pub struct ConvertArgs {
    pub cbp_path: PathBuf,
    pub output_dir: PathBuf,
    pub debug: bool,
    pub linker_type: String,
    pub test_mode: bool,
    pub ninja_path: Option<String>,
    pub no_header_insertion: bool,
}

/// 合并 compile_commands.json 命令参数
pub struct MergeCompileCommandsArgs {
    pub json_paths: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub debug: bool,
}

/// 命令行命令枚举
pub enum Command {
    /// 显示版本信息
    ShowVersion,
    /// 转换 CBP 项目
    Convert(ConvertArgs),
    /// 合并多个 compile_commands.json
    MergeCompileCommands(MergeCompileCommandsArgs),
}

/// 解析命令行参数
pub fn parse_args() -> Result<Command, Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().collect();

    // 检查是否请求帮助
    if args.len() == 2 && (args[1] == "--help" || args[1] == "-h") {
        print_convert_usage(&args[0]);
        std::process::exit(0);
    }

    // 检查并移除--debug 标志
    let debug = args.iter().any(|arg| arg == "--debug");
    if let Some(pos) = args.iter().position(|arg| arg == "--debug") {
        args.remove(pos);
    }

    // 检查是否请求显示版本
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-v") {
        return Ok(Command::ShowVersion);
    }

    // 检查是否是 merge-compile-commands 子命令
    if args.len() >= 2 && args[1] == "merge-compile-commands" {
        return parse_merge_compile_commands(args, debug);
    }

    // 默认是 convert 命令
    parse_convert(args, debug)
}

/// 解析 merge-compile-commands 子命令
fn parse_merge_compile_commands(
    mut args: Vec<String>,
    debug: bool,
) -> Result<Command, Box<dyn std::error::Error>> {
    let program_name = args[0].clone();

    // 移除子命令名（索引 1）
    args.remove(1);

    // 移除程序名（索引 0），现在只保留参数
    args.remove(0);

    // 检查并移除 --output-dir 参数
    let mut output_dir = None;
    if let Some(pos) = args.iter().position(|arg| arg == "--output-dir") {
        if pos + 1 < args.len() {
            output_dir = Some(PathBuf::from(&args[pos + 1]));
            args.remove(pos + 1);
            args.remove(pos);
        } else {
            eprintln!("Error: --output-dir requires an argument");
            print_merge_usage(&program_name);
            std::process::exit(1);
        }
    }

    // 剩余的都是 CBP 文件路径
    let cbp_paths: Vec<PathBuf> = args.into_iter().map(PathBuf::from).collect();

    if cbp_paths.is_empty() {
        eprintln!("Error: At least one CBP project file is required");
        print_merge_usage(&program_name);
        std::process::exit(1);
    }

    if cbp_paths.len() < 2 {
        eprintln!("Warning: Only one CBP file provided, nothing to merge");
    }

    // 解析每个 CBP 文件，获取 compile_commands.json 的路径
    let mut json_paths: Vec<PathBuf> = Vec::new();
    for cbp_path in &cbp_paths {
        // 检查 CBP 文件是否存在
        if !cbp_path.exists() {
            eprintln!("Warning: CBP file not found, skipping: {}", cbp_path.display());
            continue;
        }

        // 读取并解析 CBP 文件
        let xml_content = std::fs::read_to_string(cbp_path)?;
        let project_info = parse_cbp_file(&xml_content)?;

        // 获取项目目录（cbp 文件所在目录）
        let project_dir = cbp_path
            .parent()
            .unwrap_or_else(|| Path::new("."));

        // 计算 compile_commands.json 的绝对路径
        // 逻辑与 main.rs 中的 convert 命令相同
        let abs_object_output = project_dir.join(&project_info.object_output);

        // 规范化路径
        let normalized_output_dir = crate::utils::get_clean_absolute_path(
            project_dir,
            Path::new(&abs_object_output),
        );

        // 构建 compile_commands.json 路径
        let compile_commands_path = PathBuf::from(&normalized_output_dir).join("compile_commands.json");
        json_paths.push(compile_commands_path);
    }

    if json_paths.is_empty() {
        eprintln!("Error: No valid compile_commands.json paths could be determined");
        std::process::exit(1);
    }

    // 如果没有指定 output_dir，使用第一个 CBP 文件的父目录作为工作区根目录
    let output_dir = output_dir.unwrap_or_else(|| {
        // 获取第一个 CBP 文件的父目录（即项目根目录）
        cbp_paths[0]
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    });

    Ok(Command::MergeCompileCommands(MergeCompileCommandsArgs {
        json_paths,
        output_dir,
        debug,
    }))
}

/// 解析 convert 命令（原有逻辑）
fn parse_convert(
    mut args: Vec<String>,
    debug: bool,
) -> Result<Command, Box<dyn std::error::Error>> {
    let program_name = args[0].clone();
    
    // 检查是否是测试模式
    let is_test_mode = args.iter().any(|arg| arg == "--test");
    if let Some(pos) = args.iter().position(|arg| arg == "--test") {
        args.remove(pos);
    }

    // 检查是否是禁用头文件插入模式
    let no_header_insertion = args.iter().any(|arg| arg == "--no-header-insertion");
    if let Some(pos) = args.iter().position(|arg| arg == "--no-header-insertion") {
        args.remove(pos);
    }

    // 检查并移除--linker/-l参数
    let mut linker_type = "gcc".to_string();
    if let Some(linker_pos) = args.iter().position(|arg| arg == "--linker" || arg == "-l") {
        if linker_pos + 1 < args.len() {
            linker_type = args[linker_pos + 1].clone();
            args.remove(linker_pos + 1);
            args.remove(linker_pos);
        } else {
            eprintln!("Error: --linker/-l option requires an argument");
            print_convert_usage(&program_name);
            std::process::exit(1);
        }
    }

    // 检查并移除--ninja/-n参数
    let mut ninja_path = None;
    if let Some(ninja_pos) = args.iter().position(|arg| arg == "--ninja" || arg == "-n") {
        if ninja_pos + 1 < args.len() {
            ninja_path = Some(args[ninja_pos + 1].clone());
            args.remove(ninja_pos + 1);
            args.remove(ninja_pos);
        } else {
            eprintln!("Error: --ninja/-n option requires an argument");
            print_convert_usage(&program_name);
            std::process::exit(1);
        }
    }

    // 测试模式：允许 args.len() == 1
    if is_test_mode {
        return Ok(Command::Convert(ConvertArgs {
            cbp_path: PathBuf::from("--test"),
            output_dir: std::env::current_dir()?,
            debug,
            linker_type,
            test_mode: true,
            ninja_path,
            no_header_insertion: false,
        }));
    }

    // 检查是否有足够的参数
    if args.len() != 2 && args.len() != 3 {
        print_convert_usage(&program_name);
        std::process::exit(1);
    }

    let cbp_path = PathBuf::from(&args[1]);
    match std::fs::metadata(&cbp_path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                eprintln!("Path is not a file: {}", cbp_path.display());
                std::process::exit(1);
            }
        }
        Err(_) => {
            eprintln!("File not found: {}", cbp_path.display());
            std::process::exit(1);
        }
    }

    let output_dir = if args.len() == 3 {
        let output_path = PathBuf::from(&args[2]);
        // 如果输出路径是相对路径，则基于 cbp 文件的目录
        if output_path.is_relative() {
            if let Some(cbp_parent) = cbp_path.parent() {
                cbp_parent.join(output_path)
            } else {
                // 如果 cbp_path 没有父目录，则使用当前目录
                PathBuf::from(".").join(output_path)
            }
        } else {
            output_path
        }
    } else {
        cbp_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf()
    };

    Ok(Command::Convert(ConvertArgs {
        cbp_path,
        output_dir,
        debug,
        linker_type,
        test_mode: false,
        ninja_path,
        no_header_insertion,
    }))
}

/// 打印 merge-compile-commands 的使用说明
fn print_merge_usage(program: &str) {
    eprintln!(
        "Usage: {} merge-compile-commands <project1.cbp> <project2.cbp> [project3.cbp...] [--output-dir <dir>] [--debug]",
        program
    );
    eprintln!("Options:");
    eprintln!("  --output-dir <dir>  Specify workspace root directory for .clangd file");
    eprintln!("  --debug             Enable debug logging");
}

/// 打印 convert 命令的使用说明
fn print_convert_usage(program: &str) {
    eprintln!("cbp2clangd v{}", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("A tool to convert Code::Blocks project files to clangd configuration");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  {} [OPTIONS] <project.cbp> [output_dir]", program);
    eprintln!("  {} merge-compile-commands <project1.cbp> <project2.cbp> [project3.cbp...] [--output-dir <dir>] [--debug]", program);
    eprintln!("  {} --version | -v", program);
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  merge-compile-commands    Merge multiple compile_commands.json files from CBP projects");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --debug                  Enable debug logging");
    eprintln!("  --test                   Enable test mode with built-in XML content");
    eprintln!("  --no-header-insertion    Disable header insertion in clangd completion");
    eprintln!("  --linker <type>          Specify linker type (gcc or ld)");
    eprintln!("  -l <type>                Short form for --linker");
    eprintln!("  --ninja <path>           Specify custom ninja executable path");
    eprintln!("  -n <path>                Short form for --ninja");
    eprintln!("  --output-dir <dir>       Specify workspace root directory (for merge-compile-commands)");
    eprintln!("  --version, -v            Show version information");
    eprintln!("  --help, -h               Show this help message");
}
