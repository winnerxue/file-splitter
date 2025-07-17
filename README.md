# File Splitter and Restorer

A simple tool to split large files into smaller parts and restore them back to the original file, with a graphical user interface (GUI) on Windows and a command-line interface (CLI) for non-Windows systems.

## Overview

The **File Splitter and Restorer** is designed to manage large files efficiently. It allows users to split files into smaller chunks with an optional Gzip compression and restore them using JSON metadata files. The tool is tailored for Windows with a GUI, featuring a progress bar and status updates, while non-Windows users can utilize the CLI with progress indicators.

## Features

- **File Splitting**: Split large files into smaller parts based on a user-defined size limit (default 100MB).
- **Optional Compression**: Apply Gzip compression to split files to save space.
- **File Restoration**: Reconstruct original files from split parts using JSON metadata.
- **GUI for Windows**: Intuitive interface with file selection, directory picking, and progress tracking.
- **CLI for Non-Windows**: Command-line support with progress bars for splitting and restoring.
- **Cross-Platform**: Works on Windows (GUI) and other systems (CLI).

## Installation

1. **Prerequisites**:
   - Rust toolchain (install via [rustup](https://rustup.rs/)).
   - Windows for GUI, or any OS with Rust support for CLI.

2. **Clone the Repository**:
   ```bash
   git clone https://github.com/winnerxue/file-splitter.git
   cd file-splitter
   ```

3. **Build the Project**:
   - For Windows GUI:
     ```bash
     cargo build --release --features gui
     ```
   - For non-Windows CLI:
     ```bash
     cargo build --release
     ```
   The executable will be located at `target/release/file_splitter.exe` (Windows) or `target/release/file_splitter` (other OS).

## Usage

### GUI (Windows)
1. Run the executable: `target/release/file_splitter.exe`.
2. **Splitting Files**:
   - Enter file paths (comma-separated) or use "Select Files" to choose.
   - Set the split size limit (in bytes).
   - Choose an output directory and toggle compression if needed.
   - Click "Start Splitting" to begin (progress shown via bar and status).
3. **Restoring Files**:
   - Enter JSON info file paths (comma-separated) or use "Select JSON Files".
   - Set the sub-files directory and restored output directory.
   - Click "Start Restoration" to reconstruct files.

### CLI (Non-Windows)
1. Run the executable: `target/release/file_splitter`.
2. **Splitting Files**:
   ```bash
   ./file_splitter split --files file1.txt file2.txt --size-limit 104857600 --output-dir ./output --compress
   ```
   - Replace `file1.txt`, `file2.txt` with your file paths.
3. **Restoring Files**:
   ```bash
   ./file_splitter restore --info-files output/file1.json output/file2.json --input-dir ./output --output-dir ./restored
   ```
   - Adjust paths as needed.

## Notes
- The GUI window is fixed at 660x420 pixels and non-resizable.
- Ensure administrator privileges when running on Windows to access font files (e.g., `C:\Windows\Fonts\msyh.ttc`).
- JSON metadata files are generated during splitting and must be preserved for restoration.

## Contributing

Contributions are welcome! Please fork the repository, create a feature branch, and submit a pull request.

1. **Report Issues**:
   - Use the GitHub Issues page to report bugs or suggest features.
2. **Development**:
   - Ensure tests pass: `cargo test`.
   - Follow the Rust coding style guidelines.

## License

[MIT License](LICENSE) - Feel free to use and modify!