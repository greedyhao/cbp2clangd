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
    assert_eq!(project_info.source_files[0].filename, "src/chatbot.c");
    assert_eq!(project_info.source_files[0].compile, true);
    assert_eq!(project_info.source_files[0].link, true);
}

#[test]
fn test_parse_unit_compile_link_attributes() {
    // 创建一个包含compile和link属性的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Option compiler="riscv32-v2" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
            </Target>
        </Build>
        <!-- 测试普通源文件的compile和link属性 -->
        <Unit filename="src/main.c">
            <Option compile="1" link="1" />
        </Unit>
        <Unit filename="src/only_compile.c">
            <Option compile="1" link="0" />
        </Unit>
        <Unit filename="src/only_link.c">
            <Option compile="0" link="1" />
        </Unit>
        <!-- 测试特殊文件的compile和link属性 -->
        <Unit filename="src/special.asm">
            <Option compile="1" link="1" buildCommand="as $file -o $object" use="1" />
        </Unit>
        <Unit filename="src/special_no_link.asm">
            <Option compile="1" link="0" buildCommand="as $file -o $object" use="1" />
        </Unit>
        <Unit filename="src/special_no_compile.asm">
            <Option compile="0" link="1" buildCommand="as $file -o $object" use="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证源文件数量
    assert_eq!(project_info.source_files.len(), 3, "应该有3个普通源文件");
    assert_eq!(project_info.special_files.len(), 3, "应该有3个特殊文件");

    // 验证普通源文件的compile和link属性
    let main_file = project_info.source_files.iter().find(|f| f.filename == "src/main.c").expect("应该包含src/main.c");
    assert_eq!(main_file.compile, true, "src/main.c的compile属性应该为true");
    assert_eq!(main_file.link, true, "src/main.c的link属性应该为true");

    let only_compile_file = project_info.source_files.iter().find(|f| f.filename == "src/only_compile.c").expect("应该包含src/only_compile.c");
    assert_eq!(only_compile_file.compile, true, "src/only_compile.c的compile属性应该为true");
    assert_eq!(only_compile_file.link, false, "src/only_compile.c的link属性应该为false");

    let only_link_file = project_info.source_files.iter().find(|f| f.filename == "src/only_link.c").expect("应该包含src/only_link.c");
    assert_eq!(only_link_file.compile, false, "src/only_link.c的compile属性应该为false");
    assert_eq!(only_link_file.link, true, "src/only_link.c的link属性应该为true");

    // 验证特殊文件的compile和link属性
    let special_file = project_info.special_files.iter().find(|f| f.filename == "src/special.asm").expect("应该包含src/special.asm");
    assert_eq!(special_file.compile, true, "src/special.asm的compile属性应该为true");
    assert_eq!(special_file.link, true, "src/special.asm的link属性应该为true");

    let special_no_link_file = project_info.special_files.iter().find(|f| f.filename == "src/special_no_link.asm").expect("应该包含src/special_no_link.asm");
    assert_eq!(special_no_link_file.compile, true, "src/special_no_link.asm的compile属性应该为true");
    assert_eq!(special_no_link_file.link, false, "src/special_no_link.asm的link属性应该为false");

    let special_no_compile_file = project_info.special_files.iter().find(|f| f.filename == "src/special_no_compile.asm").expect("应该包含src/special_no_compile.asm");
    assert_eq!(special_no_compile_file.compile, false, "src/special_no_compile.asm的compile属性应该为false");
    assert_eq!(special_no_compile_file.link, true, "src/special_no_compile.asm的link属性应该为true");
}

#[test]
fn test_parse_special_files_compile_default() {
    // 创建一个特殊文件，没有显式指定compile属性的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Option compiler="riscv32-v2" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
            </Target>
        </Build>
        <!-- 特殊文件没有显式指定compile属性 -->
        <Unit filename="src/special.asm">
            <Option buildCommand="as $file -o $object" use="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证特殊文件数量
    assert_eq!(project_info.special_files.len(), 1, "应该有1个特殊文件");

    // 验证特殊文件的compile默认值
    let special_file = &project_info.special_files[0];
    assert_eq!(special_file.compile, false, "特殊文件的compile默认值应该为false");
    assert_eq!(special_file.link, false, "特殊文件的link默认值应该为false");
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
    assert!(
        project_info
            .global_cflags
            .contains(&"-DLE_BIS_EN=1".to_string()),
        "应该包含宏定义 -DLE_BIS_EN=1"
    );
    assert!(
        project_info
            .global_cflags
            .contains(&"-DLE_CIS_EN=1".to_string()),
        "应该包含宏定义 -DLE_CIS_EN=1"
    );

    // 验证全局编译选项数量
    assert_eq!(
        project_info.global_cflags.len(),
        2,
        "全局编译选项数量应该为2"
    );
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
    assert!(
        project_info
            .linker_lib_dirs
            .contains(&"-L../../platform/libs/net".to_string()),
        "应该包含链接库目录 -L../../platform/libs/net"
    );

    // 验证链接库是否正确解析
    assert!(
        project_info.linker_libs.contains(&"-lnet".to_string()),
        "应该包含链接库 -lnet"
    );
}

#[test]
fn test_parse_extra_commands() {
    // 创建一个包含ExtraCommands的XML内容，包含各种$变量
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Option compiler="riscv32-v2" />
        <Compiler>
            <Add option="-Wall" />
            <Add option="-g" />
        </Compiler>
        <ExtraCommands>
            <Add before='$compiler $options $includes -E -P -x c -c &quot;$(PROJECT_DIR)output\bin\copy_tone.xm&quot; -o &quot;$(PROJECT_DIR)output\bin\copy_tone.bat&quot;' />
            <Add before="Output\bin\prebuild.bat $(PROJECT_NAME)" />
        </ExtraCommands>
        <Unit filename="main.c" />
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证项目基本信息
    assert_eq!(project_info.project_name, "TestProject");
    assert_eq!(project_info.compiler_id, "riscv32-v2");

    // 验证预构建命令数量
    assert_eq!(project_info.prebuild_commands.len(), 2, "应该有2个预构建命令");

    // 验证第一个预构建命令是否包含预期内容
    let first_command = &project_info.prebuild_commands[0];
    assert!(first_command.contains("riscv32-elf-gcc"), "第一个命令应该包含编译器路径");
    assert!(first_command.contains("-Wall -g"), "第一个命令应该包含编译选项");
    assert!(first_command.contains(".\\output\\bin\\copy_tone.xm"), "第一个命令应该包含替换后的项目目录路径");
    assert!(first_command.contains(".\\output\\bin\\copy_tone.bat"), "第一个命令应该包含替换后的项目目录路径");

    // 验证第二个预构建命令是否包含预期内容
    let second_command = &project_info.prebuild_commands[1];
    assert!(second_command.contains("Output\\bin\\prebuild.bat"), "第二个命令应该包含原始路径");
    assert!(second_command.contains("TestProject"), "第二个命令应该包含项目名称");
}

#[test]
fn test_parse_unit_compile_0() {
    // 创建一个包含compile="0"属性的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
            </Target>
        </Build>
        <Unit filename="src/main.c">
            <Option compile="0" />
        </Unit>
        <Unit filename="src/helper.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证项目基本信息
    assert_eq!(project_info.project_name, "TestProject");

    // 验证源文件数量：所有普通源文件都会被添加到source_files列表中
    assert_eq!(project_info.source_files.len(), 2, "应该有2个源文件");
    
    // 验证src/main.c的compile属性为false
    let main_file = project_info.source_files.iter().find(|f| f.filename == "src/main.c").expect("应该包含src/main.c文件");
    assert_eq!(main_file.compile, false, "src/main.c的compile属性应该为false");
    
    // 验证src/helper.c的compile属性为true
    let helper_file = project_info.source_files.iter().find(|f| f.filename == "src/helper.c").expect("应该包含src/helper.c文件");
    assert_eq!(helper_file.compile, true, "src/helper.c的compile属性应该为true");
}

#[test]
fn test_parse_special_files() {
    // 创建一个包含特殊文件的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Option compiler="riscv32-v2" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
            </Target>
        </Build>
        <Unit filename="src/special.asm">
            <Option compile="1" />
            <Option compiler="riscv32-v2" buildCommand="riscv32-elf-as $options $includes $file -o $object" use="1" />
        </Unit>
        <Unit filename="src/regular.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证项目基本信息
    assert_eq!(project_info.project_name, "TestProject");
    assert_eq!(project_info.compiler_id, "riscv32-v2");

    // 验证普通源文件被正确处理
    assert_eq!(project_info.source_files.len(), 1, "应该有1个普通源文件");
    assert!(project_info.source_files.iter().any(|f| f.filename == "src/regular.c"), "应该包含src/regular.c");

    // 验证特殊文件被正确处理
    assert_eq!(project_info.special_files.len(), 1, "应该有1个特殊文件");
    let special_file = &project_info.special_files[0];
    assert_eq!(special_file.filename, "src/special.asm");
    assert_eq!(special_file.compiler_id, "riscv32-v2");
    assert_eq!(special_file.build_command, "riscv32-elf-as $options $includes $file -o $object");
}

#[test]
fn test_parse_special_files_without_build_command() {
    // 创建一个没有buildCommand的特殊文件XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Option compiler="riscv32-v2" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
            </Target>
        </Build>
        <Unit filename="src/special.asm">
            <Option compile="1" />
            <!-- 没有buildCommand属性 -->
        </Unit>
        <Unit filename="src/regular.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证项目基本信息
    assert_eq!(project_info.project_name, "TestProject");
    assert_eq!(project_info.compiler_id, "riscv32-v2");

    // 验证普通源文件被正确处理
    assert_eq!(project_info.source_files.len(), 1, "应该有1个普通源文件");
    assert!(project_info.source_files.iter().any(|f| f.filename == "src/regular.c"), "应该包含src/regular.c");

    // 验证没有buildCommand的特殊文件也被正确处理
    assert_eq!(project_info.special_files.len(), 1, "应该有1个特殊文件");
    let special_file = &project_info.special_files[0];
    assert_eq!(special_file.filename, "src/special.asm");
    assert_eq!(special_file.compiler_id, "riscv32-v2");
    assert_eq!(special_file.build_command, "", "build_command应该为空字符串");
}

#[test]
fn test_parse_march_info() {
    // 创建一个包含march指令的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
                <Compiler>
                    <Add option="-march=rv32imacxcustom" />
                </Compiler>
            </Target>
        </Build>
        <Unit filename="src/main.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证march_info被正确处理
    assert_eq!(project_info.march_info.full_march, "-march=rv32imacxcustom");
    assert_eq!(project_info.march_info.base_march, Some("-march=rv32imac".to_string()));
    assert!(project_info.march_info.has_custom_extension, "应该有自定义扩展");

    // 测试没有自定义扩展的情况
    let xml_content_no_ext = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
                <Compiler>
                    <Add option="-march=rv32imac" />
                </Compiler>
            </Target>
        </Build>
        <Unit filename="src/main.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result_no_ext = parse_cbp_file(xml_content_no_ext);
    assert!(result_no_ext.is_ok());
    let project_info_no_ext = result_no_ext.unwrap();

    // 验证没有自定义扩展的march_info
    assert_eq!(project_info_no_ext.march_info.full_march, "-march=rv32imac");
    assert_eq!(project_info_no_ext.march_info.base_march, None);
    assert!(!project_info_no_ext.march_info.has_custom_extension, "不应该有自定义扩展");
}

#[test]
fn test_parse_include_dirs() {
    // 创建一个包含include目录的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
                <Compiler>
                    <Add directory="src/include" />
                    <Add directory="../lib/include" />
                </Compiler>
            </Target>
        </Build>
        <Compiler>
            <Add directory="common/include" />
        </Compiler>
        <Unit filename="src/main.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证include_dirs被正确处理
    assert_eq!(project_info.include_dirs.len(), 3, "应该有3个包含目录");
    assert!(project_info.include_dirs.contains(&"-Isrc/include".to_string()), "应该包含-Isrc/include");
    assert!(project_info.include_dirs.contains(&"-I../lib/include".to_string()), "应该包含-I../lib/include");
    assert!(project_info.include_dirs.contains(&"-Icommon/include".to_string()), "应该包含-Icommon/include");
}

#[test]
fn test_parse_linker_options() {
    // 创建一个包含linker_options的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
            </Target>
        </Build>
        <Linker>
            <Add option="-Wl,--gc-sections" />
            <Add option="-Wl,-Map=output.map" />
            <Add option="--defsym=__stack_size=0x1000" />
        </Linker>
        <Unit filename="src/main.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证linker_options被正确处理
    assert_eq!(project_info.linker_options.len(), 3, "应该有3个链接器选项");
    assert!(project_info.linker_options.contains(&"-Wl,--gc-sections".to_string()), "应该包含-Wl,--gc-sections");
    assert!(project_info.linker_options.contains(&"-Wl,-Map=output.map".to_string()), "应该包含-Wl,-Map=output.map");
    assert!(project_info.linker_options.contains(&"--defsym=__stack_size=0x1000".to_string()), "应该包含--defsym=__stack_size=0x1000");
}

#[test]
fn test_parse_multiple_build_targets() {
    // 创建一个包含多个Build/Target节点的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/debug.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
                <Compiler>
                    <Add option="-DDEBUG=1" />
                    <Add directory="src/debug/include" />
                </Compiler>
                <Linker>
                    <Add library="debug_lib" />
                </Linker>
            </Target>
            <Target title="Release">
                <Option output="Output/bin/release.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Release" />
                <Compiler>
                    <Add option="-O2" />
                    <Add directory="src/release/include" />
                </Compiler>
                <Linker>
                    <Add library="release_lib" />
                </Linker>
            </Target>
        </Build>
        <Unit filename="src/main.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证只有第一个Build/Target节点的output和object_output被使用
    assert_eq!(project_info.output, "Output/bin/debug.elf", "应该使用第一个target的output");
    assert_eq!(project_info.object_output, "Output/obj/Debug", "应该使用第一个target的object_output");

    // 验证所有Build/Target节点的Compiler选项都被收集
    assert_eq!(project_info.global_cflags.len(), 2, "应该有2个全局编译选项");
    assert!(project_info.global_cflags.contains(&"-DDEBUG=1".to_string()), "应该包含-DDEBUG=1");
    assert!(project_info.global_cflags.contains(&"-O2".to_string()), "应该包含-O2");

    // 验证所有Build/Target节点的include目录都被收集
    assert_eq!(project_info.include_dirs.len(), 2, "应该有2个包含目录");
    assert!(project_info.include_dirs.contains(&"-Isrc/debug/include".to_string()), "应该包含-Isrc/debug/include");
    assert!(project_info.include_dirs.contains(&"-Isrc/release/include".to_string()), "应该包含-Isrc/release/include");

    // 验证所有Build/Target节点的库都被收集
    assert_eq!(project_info.linker_libs.len(), 2, "应该有2个链接库");
    assert!(project_info.linker_libs.contains(&"-ldebug_lib".to_string()), "应该包含-ldebug_lib");
    assert!(project_info.linker_libs.contains(&"-lrelease_lib".to_string()), "应该包含-lrelease_lib");
}

#[test]
fn test_parse_library_with_path() {
    // 创建一个包含带路径的库的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
                <Linker>
                    <!-- 带有相对路径的库 -->
                    <Add library="../lib/libcustom.a" />
                    <!-- 带有绝对路径的库 -->
                    <Add library="C:/path/to/lib/libabsolute.a" />
                    <!-- 普通库名 -->
                    <Add library="m" />
                </Linker>
            </Target>
        </Build>
        <Unit filename="src/main.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证库被正确处理
    assert_eq!(project_info.linker_libs.len(), 3, "应该有3个链接库");
    
    // 带相对路径的库应该直接使用完整路径
    assert!(project_info.linker_libs.contains(&"../lib/libcustom.a".to_string()), "应该包含带相对路径的库");
    
    // 带绝对路径的库应该直接使用完整路径（注意：XML中使用正斜杠，Rust代码中会保留）
    assert!(project_info.linker_libs.contains(&"C:/path/to/lib/libabsolute.a".to_string()), "应该包含带绝对路径的库");
    
    // 普通库名应该添加-l前缀
    assert!(project_info.linker_libs.contains(&"-lm".to_string()), "应该包含普通库名");
}

#[test]
fn test_parse_different_source_file_types() {
    // 创建一个包含多种类型源文件的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Build>
            <Target title="Debug">
                <Option output="Output/bin/test.elf" prefix_auto="1" extension_auto="0" />
                <Option object_output="Output/obj/Debug" />
            </Target>
        </Build>
        <!-- C源文件 -->
        <Unit filename="src/main.c">
            <Option compile="1" />
        </Unit>
        <!-- C++源文件 -->
        <Unit filename="src/helper.cpp">
            <Option compile="1" />
        </Unit>
        <!-- 汇编源文件（大写S） -->
        <Unit filename="src/startup.S">
            <Option compile="1" />
        </Unit>
        <!-- 汇编源文件（小写s） -->
        <Unit filename="src/util.s">
            <Option compile="1" />
        </Unit>
        <!-- 另一种C++源文件扩展名 -->
        <Unit filename="src/main.C">
            <Option compile="1" />
        </Unit>
        <!-- 另一种C++源文件扩展名 -->
        <Unit filename="src/main.CPP">
            <Option compile="1" />
        </Unit>
        <!-- 头文件（不应该被识别为源文件） -->
        <Unit filename="src/header.h">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证源文件数量（应该有6个源文件，头文件不算）
    assert_eq!(project_info.source_files.len(), 6, "应该有6个源文件");
    
    // 验证各种类型的源文件都被正确识别
    assert!(project_info.source_files.iter().any(|f| f.filename == "src/main.c"), "应该包含C源文件");
    assert!(project_info.source_files.iter().any(|f| f.filename == "src/helper.cpp"), "应该包含C++源文件");
    assert!(project_info.source_files.iter().any(|f| f.filename == "src/startup.S"), "应该包含大写S汇编源文件");
    assert!(project_info.source_files.iter().any(|f| f.filename == "src/util.s"), "应该包含小写s汇编源文件");
    assert!(project_info.source_files.iter().any(|f| f.filename == "src/main.C"), "应该包含大写C C++源文件");
    assert!(project_info.source_files.iter().any(|f| f.filename == "src/main.CPP"), "应该包含大写CPP C++源文件");
    
    // 验证不包含头文件
    assert!(!project_info.source_files.iter().any(|f| f.filename == "src/header.h"), "不应该包含头文件");
}

#[test]
fn test_parse_default_output_attributes() {
    // 创建一个没有output和object_output属性的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="DefaultProject" />
        <Build>
            <Target title="Debug">
                <!-- 没有output和object_output属性 -->
            </Target>
        </Build>
        <Unit filename="src/main.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证默认output和object_output被使用
    assert_eq!(project_info.output, "DefaultProject.elf", "应该使用默认output格式：<project_name>.elf");
    assert_eq!(project_info.object_output, "./", "应该使用默认object_output：./");
}

#[test]
fn test_parse_missing_object_output() {
    // 创建一个只有output属性，没有object_output属性的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Build>
            <Target title="Debug">
                <Option output="custom_output.elf" prefix_auto="1" extension_auto="0" />
                <!-- 没有object_output属性 -->
            </Target>
        </Build>
        <Unit filename="src/main.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证自定义output被使用，object_output是output的目录路径
    assert_eq!(project_info.output, "custom_output.elf", "应该使用自定义output");
    assert_eq!(project_info.object_output, "./", "应该使用output的目录路径作为object_output");
}

#[test]
fn test_parse_missing_output() {
    // 创建一个只有object_output属性，没有output属性的XML内容
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CodeBlocks_project_file>
    <FileVersion major="1" minor="6" />
    <Project>
        <Option title="TestProject" />
        <Build>
            <Target title="Debug">
                <!-- 没有output属性 -->
                <Option object_output="custom_obj_dir" />
            </Target>
        </Build>
        <Unit filename="src/main.c">
            <Option compile="1" />
        </Unit>
    </Project>
</CodeBlocks_project_file>"#;

    let result = parse_cbp_file(xml_content);
    assert!(result.is_ok());
    let project_info = result.unwrap();

    // 验证默认output被使用，自定义object_output被使用
    assert_eq!(project_info.output, "TestProject.elf", "应该使用默认output格式：<project_name>.elf");
    assert_eq!(project_info.object_output, "custom_obj_dir", "应该使用自定义object_output");
}
