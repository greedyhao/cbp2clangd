// 公共API暴露
mod cli;
mod config;
mod generator;
mod models;
mod parser;
mod utils;

// 暴露需要访问的函数
pub use cli::parse_args;
pub use config::ToolchainConfig;
pub use generator::{
    generate_build_script, generate_clangd_config, generate_clangd_fragment, generate_compile_commands, generate_ninja_build,
};
pub use parser::parse_cbp_file;
pub use utils::is_debug_mode;
pub use utils::set_debug_mode;
pub use utils::compute_absolute_path;
pub use utils::get_clean_absolute_path;
