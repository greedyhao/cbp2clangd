# cbp2clangd 架构设计

## 1. 项目概述

`cbp2clangd` 将 Code::Blocks `.cbp` 项目转换为 clangd 和 Ninja 可用的构建文件。工具链信息来自 Code::Blocks `default.conf`，不依赖硬编码的 RISC-V 工具链名称。

核心输出：

- `compile_commands.json`
- `build.ninja`
- `build.bat`
- 工作区 `.clangd`

除单项目转换外，还支持合并多个 `compile_commands.json`，以及通过 YAML 修改 Code::Blocks 工具链配置。

## 2. 模块结构

```text
main.rs
 ├─ cli.rs             命令行参数和 --target 解析
 ├─ parser.rs          CBP XML 解析
 │   └─ models.rs      ProjectInfo、BuildTarget、Unit 模型
 ├─ generator.rs       compile_commands/Ninja/build.bat/.clangd 生成
 ├─ config.rs           工具链解析和路径计算
 ├─ cb_config.rs        default.conf 读取
 ├─ config_writer.rs    default.conf 安全更新
 └─ utils.rs            路径、调试和通用工具
```

## 3. 命令行和 Target 选择

`ConvertArgs` 的主要字段：

```rust
pub struct ConvertArgs {
    pub cbp_path: PathBuf,
    pub output_dir: PathBuf,
    pub debug: bool,
    pub linker_type: String,
    pub test_mode: bool,
    pub ninja_path: Option<String>,
    pub no_header_insertion: bool,
    pub target: Option<String>,
}
```

转换命令：

```bash
cbp2clangd [OPTIONS] <project.cbp> [output_dir]
cbp2clangd --target Debug_1to3 project.cbp
```

`--target` 按 `<Target title="...">` 精确匹配。未指定时使用 XML 中的第一个 Target，以兼容旧调用；指定名称不存在时返回错误并列出可用 Target。

`cbp-build-manager` 的“构建全部 Target”模式会按 XML 顺序多次调用：

```text
cbp2clangd --target Debug project.cbp
cbp2clangd --target Debug_1to3 project.cbp
cbp2clangd --target Debug_lea project.cbp
```

## 4. CBP 解析模型

`ProjectInfo` 包含项目级编译/链接配置、Unit、Project 级 pre/post command 和有序的 Target 列表。

`BuildTarget` 包含：

```rust
pub struct BuildTarget {
    pub name: String,
    pub output: String,
    pub object_output: String,
    pub working_dir: String,
    pub target_type: Option<String>,
    pub compiler_id: Option<String>,
    pub cflags: Vec<String>,
    pub defines: Vec<String>,
    pub include_dirs: Vec<String>,
    pub linker_options: Vec<String>,
    pub linker_libs: Vec<String>,
    pub linker_lib_dirs: Vec<String>,
    pub prebuild_commands: Vec<String>,
    pub postbuild_commands: Vec<String>,
    pub march_info: MarchInfo,
}
```

解析 Target 时读取：

- `<Option output="...">`
- `<Option object_output="...">`
- `<Option working_dir="...">`
- `<Option type="...">`
- `<Option compiler="...">`
- Target 级 `<Compiler>` / `<Linker>`
- Target 级 `<ExtraCommands>`

支持展开：

- `$(TARGET_NAME)`
- `$(TARGET_OUTPUT_DIR)`
- `$(TARGET_OBJECT_DIR)`
- `$(PROJECT_NAME)`
- `$(PROJECT_DIR)`

普通 `<Unit>` 默认对所有 Target 共享；Unit 的 compile/link 属性控制是否生成编译命令和是否参与链接。

## 5. 生成器

所有生成器都有保持旧 API 的默认入口，也提供显式 Target 入口：

- `generate_compile_commands(..., target)`
- `generate_ninja_build_for_target(..., target)`
- `generate_build_script_for_target(..., target)`
- `generate_clangd_config_for_target(..., target)`
- `generate_clangd_fragment_for_target(..., target)`

选中的同一个 Target 必须贯穿所有输出，避免 compile_commands 描述一个 Target、Ninja 却使用另一个 Target。

Target 级 pre/post command 优先于 Project 级 command；如果 Target 级为空则回退到 Project 级。

全局和 Target 级链接配置按以下方式合并：

| 字段 | 合并方式 |
|---|---|
| 编译选项 | Project 级 + Target 级 |
| 包含目录 | Project 级 + Target 级 |
| 链接库 | Project 级 + Target 级，保持顺序 |
| 链接器选项 | Project 级 + Target 级 |
| 库搜索目录 | Project 级 + Target 级 |
| 输出/对象目录 | 使用选中 Target |

`.clangd` 的静态分析配置会过滤只影响代码生成的无用选项，例如 `-ffunction-sections` 和 `-fdata-sections`。

## 6. 库名规范化

`process_library_path()` 区分库名和库路径：

```text
libplatform.a       -> -lplatform
platform.a          -> -lplatform
libplatform         -> -lplatform
platform            -> -lplatform
../lib/libplatform.a -> ../lib/libplatform.a
```

无路径库使用 GCC/LD 的 `-l<name>` 语义，由链接器自动补齐 `lib` 和 `.a`。带目录的库保留为实际文件路径，由 `resolve_library_path()` 负责 Ninja 隐式依赖解析。

## 7. 工具链配置

工具链从 `%APPDATA%\CodeBlocks\default.conf` 读取：

- `MASTER_PATH`
- `C_COMPILER`
- `CPP_COMPILER`
- `LINKER`
- `LIB_LINKER`
- `INCLUDE_DIRS`
- `LIBRARY_DIRS`

解析不到 CBP 指定的编译器时直接报错并列出可用配置；不再回退到硬编码的 `riscv32-elf-*` 工具链。

## 8. 测试

测试覆盖：

- 多 Target 解析和 Target 顺序
- `--target` 选择和错误 Target
- Target 输出目录、宏和 Target 级 ExtraCommands
- 非首 Target 的 compile_commands/Ninja/build.bat 一致性
- `libplatform.a` 到 `-lplatform` 的转换
- 带路径静态库的保留和 Ninja 依赖解析
- 工具链配置和 `.clangd` 合并

运行：

```bash
cargo fmt
cargo test
```
