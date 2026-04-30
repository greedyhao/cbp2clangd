# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.1] - 2026-04-30
### Fixed
- 修复 RISC-V `-march=` 自定义扩展检测：标准扩展中的 `x`（如 `_zfinx`）不再被误判为自定义扩展
- 全局（Project/Compiler）`-march=` 配置正确传播到各 Build Target

### Changed
- `.clangd` 生成时过滤对代码分析无用的编译选项（`-ffunction-sections`、`-fdata-sections`、`-msave-restore`、`-mjump-tables-in-text`）

## [1.4.0] - 2026-04-29
### Added
- 添加 merge-compile-commands 子命令，支持合并多个 CBP 项目的 compile_commands.json
- merge-compile-commands 添加 --json 标志，支持直接合并 compile_commands.json 文件
- 添加 --output-dir 参数，用于指定 .clangd 文件的工作区根目录

### Changed
- CBP 解析改为从 Code::Blocks 的 default.conf 读取工具链配置（MASTER_PATH、INCLUDE_DIRS），支持自定义安装路径
- 识别不同 Build Target 的配置（如 Debug/Release），使用第一个 target 的 object_output

### Fixed
- CBP 模式下检查文件扩展名，非 .cbp 文件直接报错，避免误导性错误
- 修复编译问题

## [1.2.8] - 2026-01-16
### Added
- 添加 SourceFileInfo 结构，用于保存普通源文件的编译和链接信息
- 支持解析源文件的 compile 和 link 属性，控制编译和链接行为
- 支持普通源文件只链接不编译，适用于 .o 文件已存在的情况
- 特殊文件需要明确指定 compile="1"才编译，普通文件默认编译

### Fixed
- 修复了特殊文件编译命令没有运行的问题，确保所有 compile 为 true 的特殊文件都能正确触发编译

### Changed
- 修改了特殊文件的处理逻辑，将特殊文件输出作为隐式依赖添加到链接规则中，类似库文件的处理方式
- 普通文件默认链接，特殊文件默认不链接
- 更新了 README.md，添加了新功能的说明

## [1.2.7] - 2026-01-10
### Added
- 添加 --no-header-insertion 命令行参数，用于在 .clangd 配置中禁用头文件自动插入功能

## [1.2.6] - 2026-01-10
### Added
- .clangd 增加合并功能

### Fixed
- 修复target_output文件路径问题

## [1.2.5] - 2026-01-08
### Fixed
- 改进文件路径检查逻辑，避免潜在错误

## [1.2.4] - 2026-01-08
### Added
- 为特殊格式文件添加 -MMD -MF 依赖文件生成，支持头文件修改检测

## [1.2.3] - 2026-01-05
### Fixed
- 修复网络路径的处理问题
### Added
- 增加更多测试用例

## [1.2.2] - 2025-12-26
### Fixed
- windows mkdir 命令去除 -p

## [1.2.1] - 2025-12-26
### Added
- 添加单元测试

### Changed
- 重构主逻辑，支持合并多个 .clangd 配置
- compile_commands.json 输出到 output_obj 目录

## [1.2.0] - 2025-12-25
### Changed
- 合并Build/Target/Linker库和Project/Linker库，Project/Linker库放最后
- 这样可以匹配CBP的库处理顺序，确保map文件中LOAD库的位置与CBP一致

## [1.1.7] - 2025-12-20
### Changed
- 更新 README 文档，添加缺失的命令行参数说明（--test, --ninja/-n）
- 修改 ninja 路径的输入，从文件夹路径改为具体的 ninja 可执行文件路径

## [1.1.6] - 2025-12-19
### Fixed
- 修复库文件删除后不生成问题

## [1.1.5] - 2025-12-18
### Changed
- 优化体积

## [1.1.4] - 2025-12-18
### Fixed
- 处理汇编文件
- 处理依赖文件

## [1.1.3] - 2025-12-17
### Added
- 允许外部配置 ninja 路径

### Fixed
- 修复静态库的处理
- 修复cbp额外编译指令

## [1.1.2] - 2025-12-16
### Added
- 增加静态库的处理
- 增加测试用例

## [1.1.1] - 2025-12-16
### Fixed
- 格式化代码
- bat文件增加目录跳转功能

## [1.1.0] - 2025-12-16
### Added
- 从ProjectInfo获取输出文件名替代硬编码
- 添加对Build/Target/Linker库的解析并合并到链接库列表
- 添加构建脚本生成功能并增强项目解析
- 添加特殊文件构建支持
- 添加链接器类型支持

### Fixed
- 处理编译警告
- 修复输出目录的问题
- 处理链接库的顺序
- 修复构建脚本中命令执行路径问题
- 检查buildCommand不为空时再添加到build_commands
- 添加对$(TARGET_OUTPUT_DIR)变量的替换支持
- 处理带路径的库文件链接选项
- 修复对象文件路径生成以保留源文件目录结构

### Changed
- 优化特殊文件处理并添加隐式依赖
- 分离普通和特殊对象文件的处理逻辑
