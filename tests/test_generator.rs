use std::path::Path;
use cbp2clangd::{generate_ninja_build, parse_cbp_file, ToolchainConfig};

#[test]
fn test_generate_ninja_build_for_static_lib() {
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

    let project_info = parse_cbp_file(xml_content).unwrap();
    let toolchain = ToolchainConfig::from_compiler_id("riscv32-v2").unwrap();

    let result = generate_ninja_build(&project_info, Path::new("."), &toolchain);
    assert!(result.is_ok());
    let ninja_content = result.unwrap();
    
    // 打印生成的ninja内容，以便调试
    println!("Generated ninja content:\n{}", ninja_content);
    
    // 检查生成的ninja内容是否包含预期的规则和目标
    assert!(ninja_content.contains("rule ar"));
    assert!(ninja_content.contains("libchatbot.a: ar"));
    assert!(ninja_content.contains("default") && ninja_content.contains("libchatbot.a"));
}

#[test]
fn test_generate_ninja_build_for_executable() {
    // 创建一个简单的XML内容，包含可执行文件输出
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
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

    let project_info = parse_cbp_file(xml_content).unwrap();
    let toolchain = ToolchainConfig::from_compiler_id("riscv32-v2").unwrap();

    let result = generate_ninja_build(&project_info, Path::new("."), &toolchain);
    assert!(result.is_ok());
    let ninja_content = result.unwrap();
    
    // 打印生成的ninja内容，以便调试
    println!("Generated ninja content:\n{}", ninja_content);
    
    // 检查生成的ninja内容是否包含预期的规则和目标
    assert!(ninja_content.contains("rule link"));
    assert!(ninja_content.contains("build Output/bin/chatbot.elf: link"));
    assert!(ninja_content.contains("default Output/bin/chatbot.elf"));
}

#[test]
fn test_generate_ninja_build_with_target_macros() {
    // 创建一个包含Target/Compiler/Add宏定义的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Option compiler="riscv32-v2" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
                <Compiler>
                    <Add option="-DLE_BIS_EN=1" />
                    <Add option="-DLE_CIS_EN=1" />
                </Compiler>
            </Target>
        </Build>
        <Unit filename="main.c">
            <Option compiler="riscv32-v2" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let project_info = parse_cbp_file(xml_content).unwrap();
    let toolchain = ToolchainConfig::from_compiler_id("riscv32-v2").unwrap();

    let result = generate_ninja_build(&project_info, Path::new("."), &toolchain);
    assert!(result.is_ok());
    let ninja_content = result.unwrap();
    
    // 打印生成的ninja内容，以便调试
    println!("Generated ninja content with macros:\n{}", ninja_content);
    
    // 检查生成的ninja内容是否包含预期的宏定义
    assert!(ninja_content.contains("-DLE_BIS_EN=1"), "应该包含宏定义 -DLE_BIS_EN=1");
    assert!(ninja_content.contains("-DLE_CIS_EN=1"), "应该包含宏定义 -DLE_CIS_EN=1");
    
    // 检查宏定义是否被正确添加到编译规则中
    assert!(ninja_content.contains("flags = -DLE_BIS_EN=1 -DLE_CIS_EN=1"), "宏定义应该被添加到flags中");
}
