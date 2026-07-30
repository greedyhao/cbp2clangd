// 公共API暴露
mod cb_config;
mod cli;
mod config;
mod config_writer;
mod generator;
mod models;
mod parser;
mod utils;

// 暴露需要访问的函数
pub use cb_config::{
    CbCompilerConfig, CbCompilerEntry, find_default_conf, load_cb_compiler_config,
    parse_default_conf,
};
pub use cli::{ApplyConfigArgs, Command, ConvertArgs, MergeCompileCommandsArgs, parse_args};
pub use config::{ToolchainConfig, ToolchainResolveError};
pub use config_writer::{CompilerYamlConfig, apply_config_file};
pub use generator::{
    generate_build_script, generate_build_script_for_target, generate_clangd_config,
    generate_clangd_config_for_target, generate_clangd_fragment,
    generate_clangd_fragment_for_target, generate_compile_commands, generate_ninja_build,
    generate_ninja_build_for_target, merge_clangd_config, merge_compile_commands,
};
pub use parser::parse_cbp_file;
pub use utils::compute_absolute_path;
pub use utils::get_clean_absolute_path;
pub use utils::is_debug_mode;
pub use utils::set_debug_mode;
