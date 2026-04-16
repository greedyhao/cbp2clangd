# cbp2clangd 架构设计

## 1. 项目概述

`cbp2clangd` 是一个将 Code::Blocks 项目文件 (.cbp) 转换为 clangd 配置的工具，主要用于嵌入式 RISC-V 开发环境。

### 核心功能

- **CBP 转换**: 将 Code::Blocks 项目文件转换为 clangd 可用的 compile_commands.json
- **多项目合并**: 支持将多个 CBP 项目的 compile_commands.json 合并
- **配置文件生成**: 自动生成 .clangd、build.ninja、build.bat 等文件
- **动态编译器配置**: 从 Code::Blocks 的 default.conf 读取编译器路径，支持自定义安装路径

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
| 版本 | `cbp2clangd --version` |

---

### 3.2 parser.rs - CBP 文件解析

**职责**: 解析 Code::Blocks XML 格式的项目文件

**核心结构**:

```rust
pub struct ProjectInfo {
    pub compiler_id: String,       // 编译器 ID (如 riscv32-v2)
    pub project_name: String,       // 项目名称
    pub global_cflags: Vec<String>, // 全局编译选项
    pub global_include_dirs: Vec<String>,  // 全局包含目录 (-I...)
    pub source_files: Vec<SourceFileInfo>,    // 源文件列表
    pub special_files: Vec<SpecialFileBuildInfo>, // 特殊文件
    pub prebuild_commands: Vec<String>,  // 预构建命令
    pub postbuild_commands: Vec<String>, // 后构建命令
    pub targets: Vec<BuildTarget>,  // 各个Build Target的配置
    pub linker_type: String,       // 链接器类型
}
```

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
    ├── 提取 Compiler (全局选项、包含目录)
    ├── 提取 Linker (库、库目录)
    ├── 提取 Build/Target (目标配置、库、宏定义)
    ├── 提取 Unit (源文件、编译标志)
    └── 提取 ExtraCommands (预/后构建命令)
```

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

---

### 3.4 cb_config.rs - Code::Blocks 配置读取

**职责**: 从 Code::Blocks 的 `default.conf` 读取编译器配置信息

**配置文件位置**: `%APPDATA%\CodeBlocks\default.conf`

**XML 格式**:

```xml
<CodeBlocksConfig version="1">
  <compiler>
    <DEFAULT_COMPILER><str><![CDATA[gcc]]></str></DEFAULT_COMPILER>
    <sets>
      <riscv32-v2>
        <NAME><str><![CDATA[RISC-V 32-bit GCC V2]]></str></NAME>
        <MASTER_PATH><str><![CDATA[C:\path\to\toolchain]]></str></MASTER_PATH>
        <INCLUDE_DIRS><str><![CDATA[path1;path2;]]></str></INCLUDE_DIRS>
        <LIBRARY_DIRS><str><![CDATA[path1;path2;]]></str></LIBRARY_DIRS>
      </riscv32-v2>
    </sets>
  </compiler>
</CodeBlocksConfig>
```

**核心结构**:

```rust
// 单个编译器条目
pub struct CbCompilerEntry {
    pub compiler_id: String,
    pub master_path: Option<String>,     // 工具链安装根路径
    pub include_dirs: Vec<String>,       // 额外头文件目录 (分号分隔)
    pub library_dirs: Vec<String>,       // 额外库目录 (分号分隔)
}

// 编译器配置集合
pub struct CbCompilerConfig {
    pub compilers: HashMap<String, CbCompilerEntry>,
    pub default_compiler: Option<String>,
}
```

**核心函数**:

| 函数 | 说明 |
|------|------|
| `find_default_conf()` | 定位 `%APPDATA%\CodeBlocks\default.conf`，不存在返回 None |
| `parse_default_conf(xml)` | 解析 XML 内容为 `CbCompilerConfig`，使用 `roxmltree` |
| `load_cb_compiler_config()` | 便捷函数：查找并加载配置，失败静默返回 None |

**降级策略**: 如果 `default.conf` 不存在或无法读取，工具链配置将降级到 `config.rs` 中的 hardcoded 默认值，不影响正常使用。

---

### 3.5 config.rs - 工具链配置

**职责**: 管理不同编译器版本的工具链路径，支持两阶段解析

**两阶段解析流程**:

```
CBP 中的 compiler_id
         │
         ▼
┌─────────────────────────────┐
│ 1. 尝试从 default.conf 获取  │
│    master_path               │
│    (通过 cb_config.rs)       │
└──────────┬──────────────────┘
           │
     ┌─────┴─────┐
     ▼           ▼
   找到        未找到
     │           │
     │     ┌─────┴─────┐
     │     ▼           ▼
     │   有hardcoded  无hardcoded
     │   默认值?      默认值?
     │     │           │
     │   降级使用    报错退出
     │   默认路径    (列出可用编译器)
     ▼     ▼
  使用 CB 配置中的路径
```

**解析入口**:

```rust
// 推荐使用：支持 CB 配置 + 错误报告
ToolchainConfig::resolve_toolchain(compiler_id, cb_config)
    -> Result<ToolchainConfig, ToolchainResolveError>

// 旧 API：仅 hardcoded 默认值，向后兼容
ToolchainConfig::from_compiler_id(compiler_id)
    -> Option<ToolchainConfig>
```

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
| `version_name` | 版本名 (如 "V2")，从 hardcoded 映射或从 compiler_id 推导 |
| `gcc_version` | GCC 版本号 (如 "10.2.0")，仅 hardcoded 映射 |
| `toolchain_base_path` | 自定义路径，来自 default.conf 的 MASTER_PATH |
| `cb_include_dirs` | 额外 include 路径，来自 default.conf 的 INCLUDE_DIRS |

**Hardcoded 版本映射** (default.conf 不含此信息):

| Compiler ID | 版本名 | GCC 版本 | 默认路径 |
|-------------|--------|----------|----------|
| riscv32 | V1 | 6.1.0 | C:\Program Files (x86)\RV32-Toolchain\RV32-V1 |
| riscv32-v2 | V2 | 10.2.0 | C:\Program Files (x86)\RV32-Toolchain\RV32-V2 |
| riscv32-v3 | V3 | 14.2.0 | C:\Program Files (x86)\RV32-Toolchain\RV32-V3 |

**ToolchainConfig 方法**:

- `resolve_toolchain()` - 两阶段解析 (推荐入口)
- `from_compiler_id()` - 仅 hardcoded (旧 API)
- `compiler_path()` - 获取编译器路径
- `linker_path()` - 获取链接器路径
- `ar_path()` - 获取 ar 工具路径
- `include_paths()` - 获取标准 include 目录 (含 CB 额外路径)
- `is_compiler_available()` - 检查编译器是否可用

---

### 3.6 utils.rs - 工具函数

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

### 3.7 models.rs - 数据模型

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
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ cb_config.rs        │
│ 读取 default.conf   │
│ → CbCompilerConfig  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ parser.rs           │
│ 解析 CBP XML        │
│ → ProjectInfo       │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ config.rs           │
│ resolve_toolchain() │
│ → ToolchainConfig   │
│                     │
│ 优先级:             │
│  1. default.conf    │
│  2. hardcoded 默认  │
│  3. 报错退出        │
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

```
用户输入: cbp2clangd merge-compile-commands proj1.cbp proj2.cbp
         │
         ▼
┌─────────────────────┐
│ cli.rs              │
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
│ 输出到 output_dir   │
└─────────────────────┘
```

---

## 5. 依赖关系

### 5.1 外部依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| roxmltree | 0.18 | XML 解析 (CBP + default.conf) |
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
  --version, -v            显示版本信息
  --help, -h               显示帮助信息
```

### 6.2 合并命令

```bash
cbp2clangd merge-compile-commands <project1.cbp> [project2.cbp ...] [OPTIONS]

选项:
  --output-dir <dir>   指定工作区根目录（.clangd 所在目录）
  --debug              启用调试日志
```

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

如果 `default.conf` 存在且包含新编译器条目，无需修改代码。只需在 Code::Blocks 中安装并注册新编译器即可。

如果需要添加 hardcoded 版本映射（用于没有 `default.conf` 的场景），在 `config.rs` 的 `get_hardcoded_defaults()` 函数中添加新的匹配分支：

```rust
"riscv32-v4" => Some(HardcodedToolchainInfo {
    version_name: "V4".to_string(),
    gcc_version: "x.x.x".to_string(),
}),
```

### 8.2 添加新的生成器

在 `generator.rs` 中实现新的生成函数，并在 `lib.rs` 中导出。

### 8.3 自定义构建命令

通过 `Unit` 节点中的 `buildCommand` 属性支持自定义构建命令。
