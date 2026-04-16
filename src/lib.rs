// 公共API暴露
mod cb_config;
mod cli;
mod config;
mod generator;
mod models;
mod parser;
mod utils;

// 暴露需要访问的函数
pub use cb_config::{CbCompilerConfig, CbCompilerEntry, load_cb_compiler_config};
pub use cli::{parse_args, Command, ConvertArgs, MergeCompileCommandsArgs};
pub use config::{ToolchainConfig, ToolchainResolveError};
pub use generator::{
    generate_build_script, generate_clangd_config, generate_clangd_fragment, generate_compile_commands, generate_ninja_build,
    merge_clangd_config, merge_compile_commands,
};
pub use parser::parse_cbp_file;
pub use utils::is_debug_mode;
pub use utils::set_debug_mode;
pub use utils::compute_absolute_path;
pub use utils::get_clean_absolute_path;
