use std::env;
use std::path::PathBuf;

/// 命令行参数结构
pub struct CliArgs {
    pub cbp_path: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub show_version: bool,
}

/// 解析命令行参数
pub fn parse_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    // 检查是否请求显示版本
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-v") {
        return Ok(CliArgs {
            cbp_path: None,
            output_dir: None,
            show_version: true,
        });
    }
    
    // 检查是否有足够的参数
    if args.len() != 2 && args.len() != 3 {
        eprintln!("Usage: {} <project.cbp> [output_dir]", args[0]);
        eprintln!("       {} --version | -v    Show version information", args[0]);
        std::process::exit(1);
    }

    let cbp_path = PathBuf::from(&args[1]);
    if !cbp_path.is_file() {
        eprintln!("File not found: {}", cbp_path.display());
        std::process::exit(1);
    }

    let output_dir = if args.len() == 3 {
        PathBuf::from(&args[2])
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
    })
}
