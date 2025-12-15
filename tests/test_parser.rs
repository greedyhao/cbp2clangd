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
