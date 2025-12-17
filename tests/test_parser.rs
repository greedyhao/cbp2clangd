use cbp2clangd::parse_cbp_file;

#[test]
fn test_parse_cbp_file() {
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

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();
    assert_eq!(project_info.project_name, "libchatbot");
    assert_eq!(project_info.output, "Output/bin/chatbot.a");
    assert_eq!(project_info.source_files.len(), 1);
    assert_eq!(project_info.source_files[0], "src/chatbot.c");
}

#[test]
fn test_parse_target_compiler_macros() {
    // 创建一个包含Target/Compiler/Add宏定义的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Option compiler="riscv32-v2" />
        <Build>
            <Target title="Debug">
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

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();
    
    // 验证项目基本信息
    assert_eq!(project_info.project_name, "TestProject");
    assert_eq!(project_info.compiler_id, "riscv32-v2");
    
    // 验证宏定义是否被正确提取
    assert!(project_info.global_cflags.contains(&"-DLE_BIS_EN=1".to_string()), 
            "应该包含宏定义 -DLE_BIS_EN=1");
    assert!(project_info.global_cflags.contains(&"-DLE_CIS_EN=1".to_string()), 
            "应该包含宏定义 -DLE_CIS_EN=1");
    
    // 验证全局编译选项数量
    assert_eq!(project_info.global_cflags.len(), 2, 
               "全局编译选项数量应该为2");
}

#[test]
fn test_parse_target_linker_add_directory() {
    // 创建一个包含Build/Target/Linker/Add directory的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="Test" />
        <Build>
            <Target title="Debug">
                <Linker>
                    <Add library="libnet" />
                    <Add directory="../../platform/libs/net" />
                </Linker>
            </Target>
        </Build>
        <Unit filename="main.c" />
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();
    
    // 验证项目基本信息
    assert_eq!(project_info.project_name, "Test");
    
    // 验证是否正确解析了Build/Target/Linker/Add directory
    assert!(project_info.linker_lib_dirs.contains(&"-L../../platform/libs/net".to_string()),
            "应该包含链接库目录 -L../../platform/libs/net");
    
    // 验证链接库是否正确解析
    assert!(project_info.linker_libs.contains(&"-lnet".to_string()),
            "应该包含链接库 -lnet");
}
