# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
