# cbp2clangd

## 项目介绍

**cbp2clangd** 是一个用于将 Code::Blocks 项目文件（.cbp）转换为 clangd 配置文件的工具。它能够自动解析 Code::Blocks 项目中的编译器设置、包含目录和源文件，并生成对应的 `.clangd` 配置文件和 `compile_commands.json` 文件，从而使支持 clangd 的编辑器（如 VSCode、Vim、Emacs 等）能够提供更好的代码补全、导航和分析功能。

## 功能特性

- 自动解析 Code::Blocks 项目文件（.cbp）
- 提取编译器 ID、编译选项和包含目录
- 检测 RISC-V 架构相关的编译标志（如 -march）
- 生成标准化的 compile_commands.json 文件
- 生成 .clangd 配置文件，优化编辑器集成
- 生成 build.ninja 构建文件，支持 ninja 编译
- 支持多种编译器配置，自动适配不同工具链
- 支持选择链接器类型（gcc 或 ld）
- 支持解析链接脚本、链接选项和静态库配置
- 支持自定义中间文件输出目录

## 安装方法

### 前提条件

- Rust 开发环境（用于从源代码构建）

### 从源代码构建

1. 克隆项目仓库
   ```bash
   git clone https://github.com/yourusername/cbp2clangd.git
   cd cbp2clangd
   ```

2. 构建项目
   ```bash
   cargo build --release
   ```

3. 可执行文件将生成在 `target/release/` 目录下

## 使用方法

### 基本语法

```bash
cbp2clangd [--debug] [--test] [--linker <type>] [--ninja <path>] <cbp文件路径> [输出目录路径]
```

### 参数说明

- `--debug`: 启用调试日志
- `--test`: 启用测试模式，使用内置的 XML 内容
- `--linker <type>` 或 `-l <type>`: 指定链接器类型（gcc 或 ld，默认为 gcc）
- `--ninja <path>` 或 `-n <path>`: 指定自定义 ninja 可执行文件路径
- `<cbp文件路径>`: Code::Blocks 项目文件（.cbp）的路径
- `<输出目录路径>`: 生成配置文件的目标目录（通常是项目根目录）

### 查看版本信息

```bash
cbp2clangd --version
# 或使用短选项
cbp2clangd -v
```

### 使用示例

#### 示例 1：基本使用

将 app.cbp 项目转换为 clangd 配置，并输出到上级目录：

```bash
cbp2clangd app.cbp ../
```

#### 示例 2：指定使用 ld 链接器

```bash
cbp2clangd --linker ld app.cbp
```

或使用短格式：

```bash
cbp2clangd -l ld app.cbp
```

#### 示例 3：启用调试日志

```bash
cbp2clangd --debug app.cbp
```

### 生成文件

执行后，工具将生成以下文件：
- `.clangd`: clangd 的配置文件（输出到指定目录）
- `compile_commands.json`: 编译命令数据库（输出到指定目录）
- `build.ninja`: Ninja 构建文件（始终输出到 CBP 项目同目录）

## 编辑器配置

### VSCode 配置

1. 安装 [clangd 插件](https://marketplace.visualstudio.com/items?itemName=llvm-vs-code-extensions.vscode-clangd)
2. 配置本地 clangd 的路径（在插件设置中）
3. 重新加载 VSCode，插件将自动识别生成的配置文件

### 其他编辑器

对于其他支持 clangd 的编辑器（如 Vim、Emacs、Sublime Text 等），请参考各自的文档进行配置，确保编辑器能够找到生成的 `.clangd` 和 `compile_commands.json` 文件。

## 支持的编译器

工具主要针对中科蓝讯 RISC-V 架构的编译器进行了优化，支持以下编译器 ID：
- riscv32
- riscv32-v2
- riscv32-v3

对于未知的编译器 ID，工具会发出警告并回退到默认设置。

## 常见问题

### Q: 生成的配置文件无效怎么办？
A: 请检查 Code::Blocks 项目文件格式是否正确，确保项目中包含有效的编译选项和源文件。

### Q: 支持哪些文件类型？
A: 目前支持 .c .cpp .C .CPP .S .s 后缀的源文件。

### Q: 自定义扩展如何处理？
A: 工具会尝试分离 RISC-V 编译标志中的基础部分和自定义扩展部分，以优化 clangd 的处理。

## 许可证

本项目采用 MIT 许可证。

## 贡献指南

欢迎提交 Issue 和 Pull Request！

