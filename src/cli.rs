use std::env;
use std::path::PathBuf;

/// 命令行参数结构
pub struct CliArgs {
    pub cbp_path: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub show_version: bool,
    pub debug: bool,
    pub linker_type: String,
    pub test_mode: bool,
    pub ninja_path: Option<String>,
    pub no_header_insertion: bool,
}

/// 解析命令行参数
pub fn parse_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().collect();

    // 检查并移除--debug标志
    let debug = args.iter().any(|arg| arg == "--debug");
    if let Some(pos) = args.iter().position(|arg| arg == "--debug") {
        args.remove(pos);
    }

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
            eprintln!(
                "Usage: {} [--debug] [--test] [--no-header-insertion] [--linker <type>] [--ninja <path>] <project.cbp> [output_dir]",
                args[0]
            );
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
            eprintln!(
                "Usage: {} [--debug] [--test] [--no-header-insertion] [--linker <type>] [--ninja <path>] <project.cbp> [output_dir]",
                args[0]
            );
            std::process::exit(1);
        }
    }

    // 检查是否请求显示版本
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-v") {
        return Ok(CliArgs {
            cbp_path: None,
            output_dir: None,
            show_version: true,
            debug,
            linker_type,
            test_mode: false,
            ninja_path: None,
            no_header_insertion: false,
        });
    }

    // 测试模式：允许args.len() == 1
    if is_test_mode {
        return Ok(CliArgs {
            cbp_path: Some(PathBuf::from("--test")),
            output_dir: Some(std::env::current_dir()?),
            show_version: false,
            debug,
            linker_type,
            test_mode: true,
            ninja_path,
            no_header_insertion: false,
        });
    }

    // 检查是否有足够的参数
    if args.len() != 2 && args.len() != 3 {
        eprintln!(
            "Usage: {} [--debug] [--test] [--no-header-insertion] [--linker <type>] [--ninja <path>] <project.cbp> [output_dir]",
            args[0]
        );
        eprintln!(
            "       {} --version | -v    Show version information",
            args[0]
        );
        eprintln!("Options:");
        eprintln!("  --debug            Enable debug logging");
        eprintln!("  --test             Enable test mode with built-in XML content");
        eprintln!("  --no-header-insertion  Disable header insertion in clangd completion");
        eprintln!("  --linker <type>    Specify linker type (gcc or ld)");
        eprintln!("  -l <type>          Short form for --linker");
        eprintln!("  --ninja <path>     Specify custom ninja executable path");
        eprintln!("  -n <path>          Short form for --ninja");
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
        // 如果输出路径是相对路径，则基于cbp文件的目录
        if output_path.is_relative() {
            if let Some(cbp_parent) = cbp_path.parent() {
                cbp_parent.join(output_path)
            } else {
                // 如果cbp_path没有父目录，则使用当前目录
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

    Ok(CliArgs {
        cbp_path: Some(cbp_path),
        output_dir: Some(output_dir),
        show_version: false,
        debug,
        linker_type,
        test_mode: false,
        ninja_path,
        no_header_insertion,
    })
}
