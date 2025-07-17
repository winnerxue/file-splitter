# 文件分割与恢复工具

一个简单工具，用于将大文件分割成较小的部分并恢复为原始文件，Windows 系统提供图形用户界面 (GUI)，非 Windows 系统提供命令行界面 (CLI)。

## 概述

**文件分割与恢复工具**旨在高效管理大文件。它允许用户将文件分割成较小的块，并支持可选的 Gzip 压缩，使用 JSON 元数据文件进行恢复。工具专为 Windows 设计，配备带有进度条和状态更新的图形界面，而非 Windows 用户可以使用命令行界面，带有进度指示器。

## 功能

- **文件分割**：根据用户定义的大小限制（默认 100MB）将大文件分割成较小部分。
- **可选压缩**：对分割后的文件应用 Gzip 压缩以节省空间。
- **文件恢复**：使用 JSON 元数据从分割部分重建原始文件。
- **Windows 图形界面**：直观的界面，支持文件选择、目录选择和进度跟踪。
- **非 Windows 命令行界面**：支持命令行操作，包含分割和恢复的进度条。
- **跨平台**：在 Windows（GUI）和其他系统（CLI）上均可运行。

## 安装

1. **前提条件**：
   - Rust 工具链（通过 [rustup](https://rustup.rs/) 安装）。
   - Windows 用于 GUI，或任何支持 Rust 的操作系统用于 CLI。

2. **克隆仓库**：
   ```bash
   git clone https://github.com/winnerxue/file-splitter.git
   cd file-splitter
   ```

3. **构建项目**：
   - Windows GUI：
     ```bash
     cargo build --release --features gui
     ```
   - 非 Windows CLI：
     ```bash
     cargo build --release
     ```
   可执行文件将位于 `target/release/file_splitter.exe`（Windows）或 `target/release/file_splitter`（其他操作系统）。

## 使用方法

### 图形界面 (Windows)
1. 运行可执行文件：`target/release/file_splitter.exe`。
2. **分割文件**：
   - 输入文件路径（逗号分隔）或使用“选择文件”选择。
   - 设置分割大小限制（以字节为单位）。
   - 选择输出目录并根据需要切换压缩选项。
   - 点击“开始分割”开始（进度通过进度条和状态显示）。
3. **恢复文件**：
   - 输入 JSON 信息文件路径（逗号分隔）或使用“选择 JSON 文件”。
   - 设置子文件目录和恢复输出目录。
   - 点击“开始恢复”重建文件。

### 命令行界面 (非 Windows)
1. 运行可执行文件：`target/release/file_splitter`。
2. **分割文件**：
   ```bash
   ./file_splitter split --files file1.txt file2.txt --size-limit 104857600 --output-dir ./output --compress
   ```
   - 用你的文件路径替换 `file1.txt`, `file2.txt`。
3. **恢复文件**：
   ```bash
   ./file_splitter restore --info-files output/file1.json output/file2.json --input-dir ./output --output-dir ./restored
   ```
   - 根据需要调整路径。

## 注意事项
- GUI 窗口固定为 660x420 像素，不可调整大小。
- 在 Windows 上运行时，确保具有管理员权限以访问字体文件（例如 `C:\Windows\Fonts\msyh.ttc`）。
- 分割过程中会生成 JSON 元数据文件，恢复时必须保留。

## 贡献

欢迎贡献！请 fork 仓库，创建功能分支，并提交拉取请求。

1. **报告问题**：
   - 使用 GitHub Issues 页面报告错误或建议功能。
2. **开发**：
   - 确保测试通过：`cargo test`。
   - 遵循 Rust 编码风格指南。

## 许可证

[MIT 许可证](LICENSE) - 随意使用和修改！