use std::path::Path;
use cbp2clangd::{generator, parser, config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建一个简单的XML内容，包含静态库输出
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="libchatbot" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/chatbot.a" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
                <Linker>
                    <Add library="m" />
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
</CodeBlocks_project_file>"#;

    // 解析项目文件
    let mut project_info = parser::parse_cbp_file(xml_content)?;
    project_info.linker_type = "gcc".to_string();

    // 获取工具链配置
    let toolchain = config::ToolchainConfig::from_compiler_id("riscv32-v2").unwrap();

    // 生成ninja构建内容
    let ninja_content = generator::generate_ninja_build(&project_info, Path::new("."), &toolchain)?;

    // 打印生成的内容
    println!("=== Generated build.ninja content ===\n");
    println!("{}", ninja_content);
    println!("\n=== End of build.ninja content ===\n");

    Ok(())
}