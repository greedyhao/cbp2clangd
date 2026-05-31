# cbp2clangd 架构设计

## 1. 项目概述

`cbp2clangd` 是一个将 Code::Blocks 项目文件 (.cbp) 转换为 clangd 配置的工具。它通过读取 Code::Blocks 的 `default.conf` 获取编译器配置，支持任意编译器工具链（GCC、Clang、MinGW 等），无需 hardcoded 默认值。

### 核心功能

- **CBP 转换**: 将 Code::Blocks 项目文件转换为 clangd 可用的 compile_commands.json
- **多项目合并**: 支持将多个 CBP 项目的 compile_commands.json 合并
- **配置文件生成**: 自动生成 .clangd、build.ninja、build.bat 等文件
- **动态编译器配置**: 从 Code::Blocks 的 default.conf 读取编译器路径、C/C++编译器名、链接器等，支持任意工具链
- **列出编译器**: 通过 `--list-compilers` 查看系统中所有已注册的编译器及其配置

---

## 2. 模块架构

```
┌─────────────────────────────────────────────────────────────┐
│                        main.rs                               │
│                    (程序入口 & 命令分发)                      │
└─────────────────────────┬───────────────────────────────────┘
                          │
            ┌─────────────┴─────────────┐
            ▼                           ▼
    ┌───────────────┐        ┌──────────────────┐
    │ Convert 命令   │        │ Merge 命令       │
    │ 单个 CBP 转换  │        │ 多项目合并       │
    └───────┬───────┘        └────────┬─────────┘
            │                         │
            └────────────┬────────────┘
                         ▼
              ┌──────────────────────┐
              │     generator.rs     │
              │    (配置生成器)       │
              └──────────┬───────────┘
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
   ┌───────────┐   ┌───────────┐   ┌───────────┐
   │ parser.rs │   │ config.rs │   │ utils.rs  │
   │ CBP解析   │   │ 工具链配置 │   │ 工具函数  │
   └───────────┘   └─────┬─────┘   └───────────┘
                         │
                         ▼
                  ┌───────────────┐
                  │ cb_config.rs  │
                  │ CB配置读取    │ ◄─── %APPDATA%\CodeBlocks\default.conf
                  └───────────────┘
```

---

## 3. 模块详解

### 3.1 cli.rs - 命令行解析

**职责**: 解析用户输入的命令行参数

**核心结构**:

```rust
// 转换命令参数
pub struct ConvertArgs {
    pub cbp_path: PathBuf,      // CBP 文件路径
    pub output_dir: PathBuf,    // 输出目录
    pub debug: bool,            // 调试模式
    pub linker_type: String,    // 链接器类型 (gcc/ld)
    pub test_mode: bool,        // 测试模式
    pub ninja_path: Option<String>, // 自定义 ninja 路径
    pub no_header_insertion: bool, // 禁止头文件插入
}

// 合并命令参数
pub struct MergeCompileCommandsArgs {
    pub json_paths: Vec<PathBuf>,  // compile_commands.json 路径列表
    pub output_dir: PathBuf,       // 输出目录
    pub debug: bool,               // 调试模式
}
```

**命令模式**:

| 命令 | 用法 |
|------|------|
| 转换 | `cbp2clangd project.cbp [output_dir]` |
| 合并 | `cbp2clangd merge-compile-commands proj1.cbp proj2.cbp` |
| 合并 (JSON) | `cbp2clangd merge-compile-commands --json cc1.json cc2.json` |
| 版本 | `cbp2clangd --version` |
| 列出编译器 | `cbp2clangd --list-compilers` |

---

### 3.2 parser.rs - CBP 文件解析

**职责**: 解析 Code::Blocks XML 格式的项目文件

**核心结构**:

```rust
pub struct ProjectInfo {
    pub compiler_id: String,              // 编译器 ID (如 riscv32-v2)
    pub project_name: String,              // 项目名称
    pub global_cflags: Vec<String>,        // 全局编译选项 (Project/Compiler)
    pub global_include_dirs: Vec<String>,  // 全局包含目录 (Project/Compiler)
    pub global_linker_libs: Vec<String>,   // 全局链接库 (Project/Linker)
    pub global_linker_options: Vec<String>,// 全局链接器选项 (Project/Linker)
    pub global_linker_lib_dirs: Vec<String>,// 全局库搜索路径 (Project/Linker)
    pub source_files: Vec<SourceFileInfo>,        // 源文件列表
    pub special_files: Vec<SpecialFileBuildInfo>, // 特殊文件
    pub prebuild_commands: Vec<String>,     // 预构建命令
    pub postbuild_commands: Vec<String>,    // 后构建命令
    pub targets: Vec<BuildTarget>,          // 各个 Build Target 的配置
    pub linker_type: String,                // 链接器类型
}

// BuildTarget: 单个构建目标的配置
pub struct BuildTarget {
    pub name: String,                      // 目标名称 (如 Debug/Release)
    pub output: String,                    // 输出文件路径
    pub object_output: String,              // 中间文件输出目录
    pub cflags: Vec<String>,               // 编译选项 (Target/Compiler)
    pub defines: Vec<String>,              // 宏定义 (-D)
    pub include_dirs: Vec<String>,          // 包含目录 (Target/Compiler)
    pub linker_options: Vec<String>,        // 链接器选项 (Target/Linker)
    pub linker_libs: Vec<String>,           // 链接库 (Target/Linker)
    pub linker_lib_dirs: Vec<String>,       // 库搜索路径 (Target/Linker)
    pub march_info: MarchInfo,             // RISC-V -march 信息
}
```

**签名**:

```rust
pub fn parse_cbp_file(
    xml_content: &str,
    cb_config: Option<&CbCompilerConfig>,
) -> Result<ProjectInfo, Box<dyn std::error::Error>>
```

`cb_config` 为可选的 Code::Blocks default.conf 配置，仅用于 ExtraCommands 中 `$compiler` 宏的替换。当 `cb_config` 为 `None` 时，`$compiler` 替换为 compiler_id 字符串。

**解析流程**:

```
XML 内容
    │
    ▼
┌─────────────────┐
│ roxmltree 解析  │
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
提取 Project 节点
    │
    ├── 提取 Project/Option (title, compiler)
    ├── 提取 Project/Compiler → global_cflags, global_include_dirs, global_march_info
    ├── 提取 Project/Linker → global_linker_libs, global_linker_options, global_linker_lib_dirs
    ├── 提取 Build/Target (每个 Target):
    │       ├── Option → output, object_output
    │       ├── Compiler → cflags, defines, include_dirs, march_info (通过 parse_march_flag)
    │       └── Linker → linker_options, linker_libs, linker_lib_dirs
    ├── 提取 Unit (源文件、编译标志)
    └── 提取 ExtraCommands (预/后构建命令) — 使用 cb_config 解析 $compiler 宏
    
    全局 march 传播:
    └── global_march_info → 填充到未设置 march 的各个 Target
```

**`parse_march_flag` 解析规则**:

```rust
fn parse_march_flag(flag: &str, march_info: &mut MarchInfo) {
    // 仅匹配以 "_x" 开头的自定义厂商扩展
    // 标准扩展如 _zfinx 中的 'x' 不会被误判
    if let Some(x_index) = march_value.find("_x") { ... }
}
```

**合并策略**: generator 在生成 build.ninja 和 compile_commands.json 时，会将全局字段（`global_*`）与第一个 target 的字段合并使用。

---

### 3.3 generator.rs - 配置生成

**职责**: 根据解析的项目信息生成各种配置文件

**生成的文件**:

| 文件 | 位置 | 说明 |
|------|------|------|
| compile_commands.json | object_output 目录 | clangd 编译命令数据库 |
| build.ninja | 项目根目录 | Ninja 构建脚本 |
| build.bat | 项目根目录 | Windows 构建批处理 |
| .clangd | 工作区根目录 | clangd 配置文件 |

**核心函数**:

- `generate_compile_commands()` - 生成 compile_commands.json
- `generate_ninja_build()` - 生成 Ninja 构建脚本
- `generate_build_script()` - 生成 Windows 批处理脚本
- `generate_clangd_config()` - 生成 .clangd 基础配置
- `generate_clangd_fragment()` - 生成 .clangd 项目片段
- `merge_clangd_config()` - 合并 .clangd 配置
- `merge_compile_commands()` - 合并多个 compile_commands.json

**`.clangd` 无用选项过滤**:

`generate_clangd_config()` 在构建 `CompileFlags.Add` 列表时，会跳过对 clangd 代码分析无用的编译选项：

```rust
let skip_add_flags: HashSet&str> = [
    "-ffunction-sections",
    "-fdata-sections",
    "-msave-restore",
    "-mjump-tables-in-text",
].iter().cloned().collect();
```

这些选项影响链接或代码生成，对静态分析和补全没有帮助，因此不会写入 `.clangd`。

**多 Target 合并策略**:

generator 使用第一个 target（通常是 Debug）的配置进行生成。全局字段与 target 字段在链接阶段合并：

| 字段 | 全局来源 | Target 来源 | 合并方式 |
|------|----------|-------------|----------|
| 链接库 | `global_linker_libs` | `target.linker_libs` | `.chain()` 拼接 |
| 链接器选项 | `global_linker_options` | `target.linker_options` | 循环遍历 |
| 库搜索路径 | `global_linker_lib_dirs` | `target.linker_lib_dirs` | 循环遍历 |
| 输出文件 | - | `target.output` | 直接使用 |
| 中间目录 | - | `target.object_output` | 直接使用 |

---

### 3.4 cb_config.rs - Code::Blocks 配置读取

**职责**: 从 Code::Blocks 的 `default.conf` 读取编译器配置信息

**配置文件位置**: `%APPDATA%\CodeBlocks\default.conf`

**XML 格式**（含 `<user_sets>`）：

```xml
<CodeBlocksConfig version="1">
  <compiler>
    <DEFAULT_COMPILER><str><![CDATA[gcc]]></str></DEFAULT_COMPILER>
    <sets>
      <riscv32_v2>
        <NAME><str><![CDATA[RISCV32-V2]]></str></NAME>
        <MASTER_PATH><str><![CDATA[C:\path\to\toolchain]]></str></MASTER_PATH>
        <C_COMPILER><str><![CDATA[riscv32-elf-gcc.exe]]></str></C_COMPILER>
        <CPP_COMPILER><str><![CDATA[riscv32-elf-g++.exe]]></str></CPP_COMPILER>
        <LINKER><str><![CDATA[riscv32-elf-ld.exe]]></str></LINKER>
        <LIB_LINKER><str><![CDATA[riscv32-elf-ar.exe]]></str></LIB_LINKER>
        <INCLUDE_DIRS><str><![CDATA[path1;path2;]]></str></INCLUDE_DIRS>
        <LIBRARY_DIRS><str><![CDATA[path1;path2;]]></str></LIBRARY_DIRS>
      </riscv32_v2>
    </sets>
    <user_sets>
      <!-- 用户自定义的编译器覆盖，优先级高于 sets -->
      <riscv32>
        <NAME><str><![CDATA[RISCV32]]></str></NAME>
        <MASTER_PATH><str><![CDATA[...]]></str></MASTER_PATH>
      </riscv32>
    </user_sets>
  </compiler>
</CodeBlocksConfig>
```

**核心结构**：

```rust
// 单个编译器条目
pub struct CbCompilerEntry {
    pub compiler_id: String,
    pub name: Option<String>,            // 编译器显示名称 (NAME)
    pub master_path: Option<String>,     // 工具链安装根路径 (MASTER_PATH)
    pub c_compiler: Option<String>,      // C 编译器可执行文件名 (C_COMPILER)
    pub cpp_compiler: Option<String>,    // C++ 编译器可执行文件名 (CPP_COMPILER)
    pub linker: Option<String>,          // 链接器可执行文件名 (LINKER)
    pub lib_linker: Option<String>,      // 库管理器可执行文件名 (LIB_LINKER)
    pub include_dirs: Vec<String>,       // 额外头文件目录 (INCLUDE_DIRS，分号分隔)
    pub library_dirs: Vec<String>,       // 额外库目录 (LIBRARY_DIRS，分号分隔)
}

// 编译器配置集合
pub struct CbCompilerConfig {
    pub compilers: HashMap<String, CbCompilerEntry>,
    pub default_compiler: Option<String>,
}
```

**核心函数**：

| 函数 | 说明 |
|------|------|
| `find_default_conf()` | 定位 `%APPDATA%\CodeBlocks\default.conf`，不存在返回 None |
| `parse_default_conf(xml)` | 解析 XML 为 `CbCompilerConfig`，同时读取 `<sets>` 和 `<user_sets>`，后者覆盖前者同名条目 |
| `load_cb_compiler_config()` | 便捷函数：查找并加载配置，失败返回 None |
| `parse_compiler_sets(node, tag, map)` | 内部函数：从 `<sets>` 或 `<user_sets>` 中提取编译器条目 |

---

### 3.5 config_writer.rs - 编译器配置写入

**职责**: 通过 YAML 配置文件添加或更新 Code::Blocks `default.conf` 中的编译器条目。

**YAML 格式**:

```yaml
compilers:
  - name: "RISCV32-V4"               # → compiler_id: riscv32_v4
    master_path: "C:\\toolchain\\v4"
    c_compiler: "riscv32-elf-gcc.exe"   # 可选
    cpp_compiler: "riscv32-elf-g++.exe" # 可选
    linker: "riscv32-elf-ld.exe"        # 可选
    lib_linker: "riscv32-elf-ar.exe"    # 可选
    parent: "gcc"                       # 可选，默认 gcc
```

**核心函数**:

| 函数 | 说明 |
|------|------|
| `name_to_compiler_id(name)` | NAME → compiler_id：小写 + 连字符/空格 → 下划线 |
| `generate_entry_xml(entry, id)` | 生成匹配 `default.conf` 缩进风格的 XML 片段（含 CDATA） |
| `find_entry_in_content(content, id)` | 在文本中查找已有条目位置 |
| `ensure_user_sets(content)` | 确保 `<user_sets>` 标签存在并返回插入点 |
| `apply_config_to_content(content, config)` | 将 YAML 配置应用到文本内容（更新或插入） |
| `apply_config_file(yaml_path, conf_path)` | 完整流程：读取 YAML → 解析/备份/修改/写回 `default.conf` |

**工作流程**:

```
apply-config <config.yaml>
    │
    ▼
┌─────────────────────────┐
│ 读取 YAML               │
│ 解析配置列表             │
└──────────┬──────────────┘
           ▼
┌─────────────────────────┐
│ 对每个配置项:            │
│ 1. name → compiler_id   │
│ 2. 生成 XML 片段         │
│ 3. 在 default.conf 中    │
│    查找 compiler_id      │
│    ├─ 存在 → 替换条目     │
│    └─ 不存在 → 插入到     │
│        <user_sets> 中    │
└──────────┬──────────────┘
           ▼
┌─────────────────────────┐
│ 验证 XML 合法性           │
│ 备份 → 写回 default.conf │
└─────────────────────────┘
```

---

### 3.6 config.rs - 工具链配置

**职责**: 根据 compiler_id 从 `default.conf` 解析工具链路径，构造 C/C++ 编译器、链接器和库管理器的完整路径。

**解析流程**:

```
CBP 中的 compiler_id (如 "RISCV32-V2")
         │
         ▼
┌───────────────────────────────────┐
│ 1. 精确匹配 XML 标签名            │
│    ("riscv32_v2")                 │
└──────────┬────────────────────────┘
           │
      ┌────┴────┐
      ▼         ▼
    命中       未命中
      │         │
      │         ▼
      │  ┌──────────────────────────┐
      │  │ 2. 不区分大小写匹配 NAME  │
      │  │    ("RISCV32-V2"         │
      │  │     → 条目名 riscv32_v2, │
      │  │       NAME="RISCV32-V2") │
      │  └──────────┬───────────────┘
      │        ┌────┴────┐
      │        ▼         ▼
      │      命中       未命中
      │        │         │
      │        │         ▼
      │        │    ┌────────────┐
      │        │    │ 报错退出   │
      │        │    │ (列出可用  │
      │        │    │  编译器)   │
      │        │    └────────────┘
      ◄────────┘
      │
      ▼
┌───────────────────────────────────┐
│ 验证 MASTER_PATH 是否存在         │
│ 不存在 → 报错退出                  │
└──────────┬────────────────────────┘
           │
           ▼
┌───────────────────────────────────┐
│ ToolchainConfig {                  │
│   toolchain_base_path,             │
│   c_compiler,    // 或默认 gcc.exe │
│   cpp_compiler,  // 或默认 g++.exe │
│   linker,        // 或默认 ld.exe  │
│   lib_linker,    // 或默认 ar.exe  │
│   cb_include_dirs                  │
│ }                                   │
└───────────────────────────────────┘
```

**解析入口**:

```rust
// 唯一入口：从 default.conf 解析，找不到则报错
ToolchainConfig::resolve_toolchain(compiler_id, cb_config)
    -> Result<ToolchainConfig, ToolchainResolveError>
```

编译器 ID 的查找顺序：
1. 精确匹配 XML 标签名（HashMap key）
2. 遍历所有条目，不区分大小写匹配 `<NAME>` 字段

**错误类型**:

```rust
pub enum ToolchainResolveError {
    UnknownCompiler {
        compiler_id: String,
        available: Vec<String>,  // 列出所有可用编译器
    },
}
```

**ToolchainConfig 字段**:

| 字段 | 说明 |
|------|------|
| `toolchain_base_path` | 工具链基础路径，来自 default.conf 的 MASTER_PATH，必须存在 |
| `c_compiler` | C 编译器可执行文件名（如 `riscv32-elf-gcc.exe`），默认 `gcc.exe` |
| `cpp_compiler` | C++ 编译器可执行文件名，默认 `g++.exe` |
| `linker` | 链接器可执行文件名，默认 `ld.exe` |
| `lib_linker` | 库管理器可执行文件名，默认 `ar.exe` |
| `cb_include_dirs` | 额外 include 路径，来自 default.conf 的 INCLUDE_DIRS |

**ToolchainConfig 方法**:

| 方法 | 说明 |
|------|------|
| `resolve_toolchain(id, config)` | 从 default.conf 解析工具链（唯一入口） |
| `compiler_path()` | `{base}\bin\{C_COMPILER 或 gcc.exe}` |
| `cpp_compiler_path()` | `{base}\bin\{CPP_COMPILER 或 g++.exe}` |
| `linker_path(type)` | `type="ld"` → `{base}\bin\{LINKER 或 ld.exe}`；否则等同 `compiler_path()` |
| `ar_path()` | `{base}\bin\{LIB_LINKER 或 ar.exe}` |
| `include_paths()` | 仅返回 `cb_include_dirs`（调用方自行添加 `-I` 前缀） |
| `is_compiler_available()` | 检查 `compiler_path()` 可执行文件是否存在 |

---

### 3.7 utils.rs - 工具函数

**职责**: 提供路径处理和 Windows API 调用等工具函数

**核心功能**:

- `compute_absolute_path()` - 计算绝对路径（避免 UNC 路径问题）
- `get_clean_absolute_path()` - 逻辑解析路径（不依赖文件系统）
- `get_short_path()` - 获取 Windows 8.3 短路径（处理空格问题）
- `quote_if_needed()` - 路径加引号（处理空格）
- `escape_ninja_path()` - Ninja 路径转义
- `set_debug_mode()` / `is_debug_mode()` - 调试模式控制
- `debug_println!` - 条件打印宏

---

### 3.8 models.rs - 数据模型

**职责**: 定义项目中使用的核心数据结构

```rust
// 源文件信息
pub struct SourceFileInfo {
    pub filename: String,  // 文件名
    pub compile: bool,      // 是否编译
    pub link: bool,         // 是否链接
}

// 特殊文件构建信息
pub struct SpecialFileBuildInfo {
    pub filename: String,
    pub compiler_id: String,
    pub build_command: String,
    pub compile: bool,
    pub link: bool,
}

// 编译命令 (用于 compile_commands.json)
pub struct CompileCommand {
    pub directory: String,  // 工作目录
    pub command: String,    // 编译命令
    pub file: String,       // 源文件路径
}

// RISC-V 架构信息
pub struct MarchInfo {
    pub full_march: String,        // 完整 -march 参数
    pub base_march: Option<String>, // 基础架构
    pub has_custom_extension: bool, // 是否包含自定义扩展
}
```

---

## 4. 数据流

### 4.1 单项目转换流程

```
用户输入: cbp2clangd project.cbp
         │
         ▼
┌─────────────────────┐
│ main.rs             │
│ 解析命令行参数       │
│ 读取 default.conf   │
│ → CbCompilerConfig  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ parser.rs           │
│ 解析 CBP XML        │
│ (传入 cb_config     │
│  用于 $compiler 宏) │
│ → ProjectInfo       │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ config.rs           │
│ resolve_toolchain() │
│ → ToolchainConfig   │
│                     │
│ 从 default.conf     │
│ 精确/NANE匹配查找   │
│ 未找到 → 报错退出   │
│ (无 hardcoded 回退) │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ generator.rs        │
│ 生成配置文件         │
│  - compile_commands │
│  - build.ninja      │
│  - build.bat        │
│  - .clangd          │
└─────────────────────┘
```

### 4.2 多项目合并流程

**CBP 模式** (默认):
```
用户输入: cbp2clangd merge-compile-commands proj1.cbp proj2.cbp
         │
         ▼
┌─────────────────────┐
│ cli.rs              │
│ 检查 .cbp 扩展名    │
│ 解析每个 CBP        │
│ 获取 compile_commands.json 路径 │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ generator.rs        │
│ merge_compile_      │
│ commands()          │
│                     │
│ 合并 JSON 数组      │
│ 写回第一个 JSON     │
│ 生成 .clangd        │
└─────────────────────┘
```

**JSON 模式** (`--json`):
```
用户输入: cbp2clangd merge-compile-commands --json cc1.json cc2.json
         │
         ▼
┌─────────────────────┐
│ cli.rs              │
│ 跳过 CBP 解析       │
│ 直接使用 JSON 路径  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ generator.rs        │
│ merge_compile_      │
│ commands()          │
│                     │
│ 合并 JSON 数组      │
│ 写回第一个 JSON     │
│ .clangd 写入第一个  │
│ JSON 的父目录       │
└─────────────────────┘
```

---

## 5. 依赖关系

### 5.1 外部依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| roxmltree | 0.21.1 | XML 解析 (CBP + default.conf) |
| serde_json | 1.0 | JSON 序列化/反序列化 |
| windows-sys | 0.52 | Windows API 调用 |

### 5.2 模块依赖图

```
main.rs
  │
  ├─► cli.rs (parse_args)
  │
  ├─► cb_config.rs (load_cb_compiler_config)
  │
  ├─► parser.rs (parse_cbp_file)
  │
  ├─► config.rs (ToolchainConfig::resolve_toolchain)
  │
  ├─► config_writer.rs (apply_config_file)
  │
  └─► generator.rs
          │
          ├─► parser.rs (ProjectInfo)
          ├─► config.rs (ToolchainConfig)
          ├─► models.rs (CompileCommand)
          └─► utils.rs (路径处理函数)

cli.rs
  │
  ├─► parser.rs (parse_cbp_file)
  │
  └─► utils.rs (get_clean_absolute_path)

cb_config.rs
  │
  └─► utils.rs (debug_println!)

parser.rs
  │
  ├─► config.rs (ToolchainConfig)
  │
  └─► models.rs (数据结构)

config.rs
  │
  ├─► cb_config.rs (CbCompilerConfig)
  │
  └─► utils.rs (debug_println!)

config_writer.rs
  │
  └─► (文本操作 default.conf)

utils.rs
  │
  └─► windows-sys (GetShortPathNameW)

models.rs
  │
  └─► (无外部依赖)
```

---

## 6. 命令行接口

### 6.1 转换命令

```bash
cbp2clangd [OPTIONS] <project.cbp> [output_dir]

选项:
  --debug                  启用调试日志
  --test                   启用测试模式（内置 XML）
  --no-header-insertion    禁用 clangd 头文件自动插入
  --linker <type>          指定链接器类型 (gcc 或 ld)
  -l <type>                --linker 简写
  --ninja <path>           指定自定义 ninja 路径
  -n <path>                --ninja 简写
  --list-compilers         列出 Code::Blocks 中已注册的所有编译器配置
  --version, -v            显示版本信息
  --help, -h               显示帮助信息
```

### 6.2 合并命令

```bash
cbp2clangd merge-compile-commands [--json] <file1> <file2> ... [OPTIONS]

选项:
  --json               直接合并 compile_commands.json 文件（跳过 CBP 解析）
  --output-dir <dir>   指定工作区根目录（.clangd 所在目录，CBP 模式专用）
  --debug              启用调试日志
```

**CBP 模式**（默认）：输入 `.cbp` 文件，工具自动从 CBP 的 target 配置中定位 `compile_commands.json` 路径并进行合并。非 `.cbp` 文件会报错退出。

**JSON 模式**（`--json`）：输入文件直接作为 `compile_commands.json` 路径，合并结果写入第一个 JSON 文件，`.clangd` 写入其父目录。此模式下不允许使用 `--output-dir`。

### 6.3 应用编译器配置

```bash
cbp2clangd apply-config <config.yaml>
```

通过 YAML 文件添加或更新 Code::Blocks 的 `default.conf` 中的编译器条目。

```yaml
compilers:
  - name: "RISCV32-V4"
    master_path: "C:\\toolchain\\v4"
    c_compiler: "riscv32-elf-gcc.exe"
    cpp_compiler: "riscv32-elf-g++.exe"
    linker: "riscv32-elf-ld.exe"
    lib_linker: "riscv32-elf-ar.exe"
    parent: "gcc"
```

- compiler_id 由 `name` 自动生成：小写 + 连字符/空格 → 下划线（如 `RISCV32-V4` → `riscv32_v4`）
- 已有配置更新，新配置插入到 `<user_sets>` 中
- 原 `default.conf` 自动备份为 `.conf.bak`

---

## 7. 输出文件说明

### 7.1 compile_commands.json

JSON 格式的编译命令数据库，供 clangd 用于代码补全、导航等。

```json
[
  {
    "directory": "C:\\project\\obj\\Debug",
    "command": "C:\\...\\riscv32-elf-gcc.exe -c -o object -Wall -g source.c",
    "file": "C:\\project\\src\\source.c"
  }
]
```

### 7.2 build.ninja

Ninja 构建系统的构建脚本，定义编译规则和构建目标。

### 7.3 build.bat

Windows 批处理脚本，简化构建流程。

### 7.4 .clangd

clangd 配置文件，支持多项目片段。

```yaml
CompileFlags:
  Add: [-std=c11, -Wall]

---
PathMatch: project1/.*
CompileFlags:
  Add: [-Iproject1/include]

---
PathMatch: project2/.*
CompileFlags:
  Add: [-Iproject2/include]
```

---

## 8. 扩展点

### 8.1 添加新编译器支持

工具链信息完全来自 Code::Blocks 的 `default.conf`（`<sets>` 和 `<user_sets>`）。只需在 Code::Blocks 中安装并注册新编译器，`default.conf` 中就会有对应的条目。

`default.conf` 条目必须包含 `MASTER_PATH`，可选包含 `C_COMPILER`、`CPP_COMPILER`、`LINKER`、`LIB_LINKER` 字段：

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `MASTER_PATH` | 工具链安装根路径 | **必填** |
| `C_COMPILER` | C 编译器可执行文件名 | `gcc.exe` |
| `CPP_COMPILER` | C++ 编译器可执行文件名 | `g++.exe` |
| `LINKER` | 链接器可执行文件名 | `ld.exe` |
| `LIB_LINKER` | 库管理器可执行文件名 | `ar.exe` |

`config.rs` 中不再维护 hardcoded 默认值，所有编译器配置均来自 `default.conf`。

### 8.2 添加新的生成器

在 `generator.rs` 中实现新的生成函数，并在 `lib.rs` 中导出。

### 8.3 自定义构建命令

通过 `Unit` 节点中的 `buildCommand` 属性支持自定义构建命令。
