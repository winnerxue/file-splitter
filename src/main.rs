// src/main.rs
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

// Common imports for both CLI and GUI
use anyhow::Result;
use std::path::PathBuf; // Removed Path as it was unused
use std::fs; 

// --- CLI specific imports and logic ---
#[cfg(not(target_os = "windows"))] // This block compiles only if NOT targeting Windows
mod cli {
    use super::*; // Import common items from outer scope
    use clap::{Parser, Subcommand};
    use indicatif::{ProgressBar, ProgressStyle};
    use file_splitter::split_single_file; // Import from our lib
    use file_splitter::restore_single_file; // Import from our lib
    use file_splitter::SplitInfo; // Import from our lib
    use serde_json; // For parsing SplitInfo from JSON
    use anyhow::Context; // <--- ADD THIS LINE

    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    pub struct Cli {
        #[command(subcommand)]
        pub command: Commands,
    }

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Split one or more files
        Split {
            /// List of file paths to split
            #[arg(required = true)]
            files: Vec<PathBuf>,
            
            /// Split size limit (bytes). If file size is greater than this, it will be split. Default 100MB (104857600 bytes)
            #[arg(short, long, default_value = "104857600")]
            size_limit: u64,
            
            /// Root directory where split sub-files and info files will be stored
            #[arg(short, long, default_value = ".")]
            output_dir: PathBuf,

            /// Whether to Gzip compress the split sub-files
            #[arg(short, long)]
            compress: bool,
        },
        
        /// Restore one or more files
        Restore {
            /// List of split info JSON file paths (e.g., my_file_parts/my_file.json)
            #[arg(required = true)]
            info_files: Vec<PathBuf>,
            
            /// Root directory where the split sub-files are located (usually the same as the output_dir during split)
            #[arg(short, long, default_value = ".")]
            input_dir: PathBuf,
            
            /// Directory where the restored large files will be saved
            #[arg(short, long, default_value = ".")]
            output_dir: PathBuf,
        },
    }

    pub fn run_cli() -> Result<()> {
        let cli = Cli::parse();

        match &cli.command {
            Commands::Split { files, size_limit, output_dir, compress } => {
                println!("\nStarting to process {} files for splitting...", files.len());
                for file_path in files {
                    println!("\nProcessing file: {}", file_path.display());
                    let progress = ProgressBar::new(0); // Placeholder, actual total will be set by callback
                    progress.set_style(ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                        .unwrap());
                    
                    let progress_cb = {
                        let progress = progress.clone();
                        move |current, total| {
                            if progress.length().is_none() || progress.length().unwrap() != total {
                                progress.set_length(total);
                            }
                            progress.set_position(current);
                        }
                    };

                    let message_cb = {
                        let progress = progress.clone();
                        move |msg: String| {
                            progress.set_message(msg);
                        }
                    };

                    split_single_file(
                        file_path,
                        *size_limit,
                        output_dir,
                        *compress,
                        Some(Box::new(progress_cb)),
                        Some(Box::new(message_cb)), // <--- WRAP IN Box::new()
                    )?;
                    progress.finish_with_message(format!("'{}' splitting complete", file_path.display()));
                }
                println!("\nAll files split successfully!");
                println!("Each original file's split information (e.g., 'filename.json') is saved within its dedicated subdirectory (e.g., 'output_dir/filename_parts/').");
            }
            Commands::Restore { info_files, input_dir, output_dir } => {
                println!("\nStarting to restore {} files...", info_files.len());
                for info_file_path in info_files {
                    println!("\nReading restore info file: {}", info_file_path.display());
                    
                    let metadata_content = fs::read_to_string(info_file_path)
                        .context(format!("Failed to read restore info file: {}", info_file_path.display()))?; // <--- CHANGED with_context TO context AND REMOVED CLOSURE
                    
                    let file_info: SplitInfo = serde_json::from_str(&metadata_content)
                        .context(format!("Failed to parse restore info JSON file: {}", info_file_path.display()))?; // <--- CHANGED with_context TO context AND REMOVED CLOSURE

                    let progress = ProgressBar::new(0); // Placeholder
                    progress.set_style(ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                        .unwrap());
                    
                    let progress_cb = {
                        let progress = progress.clone();
                        move |current, total| {
                            if progress.length().is_none() || progress.length().unwrap() != total {
                                progress.set_length(total);
                            }
                            progress.set_position(current);
                        }
                    };

                    let message_cb = {
                        let progress = progress.clone();
                        move |msg: String| {
                            progress.set_message(msg);
                        }
                    };

                    restore_single_file(
                        &file_info,
                        input_dir,
                        output_dir,
                        Some(Box::new(progress_cb)),
                        Some(Box::new(message_cb)), // <--- WRAP IN Box::new()
                    )?;
                    progress.finish_with_message(format!("'{}' restoration complete", file_info.original_filename));
                }
                println!("\nAll files restored successfully!");
            }
        }
        Ok(())
    }
}

// --- GUI specific imports and logic (for Windows) ---
#[cfg(target_os = "windows")]
mod gui {
    use super::*;
    use eframe::{egui, NativeOptions};
    use std::sync::mpsc::{self, Sender, Receiver};
    use std::thread;
    use file_splitter::split_single_file;
    use file_splitter::restore_single_file;
    use file_splitter::SplitInfo;
    use rfd::FileDialog;
    use serde_json;

    // Messages sent from worker thread to GUI thread
    enum WorkerMessage {
        Progress(u64, u64),
        Message(String),
        Error(String),
        Done,
    }

    #[derive(Default)]
    pub struct FileSplitterApp {
        split_files_input: String,
        split_size_limit: String,
        split_output_dir: String,
        split_compress: bool,
        restore_info_files_input: String,
        restore_input_dir: String,
        restore_output_dir: String,
        current_progress: f32,
        status_message: String,
        is_processing: bool,
        tx: Option<Sender<WorkerMessage>>,
        rx: Option<Receiver<WorkerMessage>>,
    }

    impl eframe::App for FileSplitterApp {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("File Splitter and Restorer");
                ui.separator();

                ui.group(|ui| {
                    ui.heading("File Splitting");
                    ui.horizontal(|ui| {
                        ui.label("Files to split (comma-separated):");
                        ui.text_edit_singleline(&mut self.split_files_input);
                        if ui.button("Select Files").clicked() {
                            if let Some(paths) = FileDialog::new().pick_files() {
                                for path in &paths {
                                    println!("Raw PathBuf: {:?}", path);
                                    println!("Converted Path (to_string_lossy): {}", path.to_string_lossy());
                                    if let Some(s) = path.to_str() {
                                        println!("Converted Path (to_str): {}", s);
                                    } else {
                                        println!("Path contains invalid UTF-8 sequence");
                                    }
                                }
                                self.split_files_input = paths.iter()
                                    .map(|p| p.to_string_lossy().into_owned())
                                    .collect::<Vec<_>>()
                                    .join(",");
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Split size limit (bytes):");
                        ui.text_edit_singleline(&mut self.split_size_limit);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Output Directory:");
                        ui.text_edit_singleline(&mut self.split_output_dir);
                        if ui.button("Select Directory").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                self.split_output_dir = path.to_string_lossy().into_owned();
                            }
                        }
                    });
                    ui.checkbox(&mut self.split_compress, "Compress Sub-files (Gzip)");

                    if ui.add_enabled(!self.is_processing, egui::Button::new("Start Splitting")).clicked() {
                        self.start_operation(ctx.clone(), OperationType::Split);
                    }
                });

                ui.separator();

                ui.group(|ui| {
                    ui.heading("File Restoration");
                    ui.horizontal(|ui| {
                        ui.label("Split Info JSON Files (comma-separated):");
                        ui.text_edit_singleline(&mut self.restore_info_files_input);
                        if ui.button("Select JSON Files").clicked() {
                            if let Some(paths) = FileDialog::new().add_filter("JSON Files", &["json"]).pick_files() {
                                self.restore_info_files_input = paths.iter()
                                    .map(|p| p.to_string_lossy().into_owned())
                                    .collect::<Vec<_>>()
                                    .join(",");
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Sub-files Directory:");
                        ui.text_edit_singleline(&mut self.restore_input_dir);
                        if ui.button("Select Directory").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                self.restore_input_dir = path.to_string_lossy().into_owned();
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Restored Output Directory:");
                        ui.text_edit_singleline(&mut self.restore_output_dir);
                        if ui.button("Select Directory").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                self.restore_output_dir = path.to_string_lossy().into_owned();
                            }
                        }
                    });

                    if ui.add_enabled(!self.is_processing, egui::Button::new("Start Restoration")).clicked() {
                        self.start_operation(ctx.clone(), OperationType::Restore);
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Status:");
                    ui.label(&self.status_message);
                });
                ui.add(egui::ProgressBar::new(self.current_progress).show_percentage());

                if let Some(rx) = &self.rx {
                    while let Ok(msg) = rx.try_recv() {
                        match msg {
                            WorkerMessage::Progress(current, total) => {
                                self.current_progress = if total > 0 { current as f32 / total as f32 } else { 0.0 };
                                ctx.request_repaint();
                            }
                            WorkerMessage::Message(msg) => {
                                self.status_message = msg;
                                ctx.request_repaint();
                            }
                            WorkerMessage::Error(err) => {
                                self.status_message = format!("Error: {}", err);
                                self.is_processing = false;
                                ctx.request_repaint();
                            }
                            WorkerMessage::Done => {
                                self.status_message = "Operation Complete!".to_string();
                                self.is_processing = false;
                                self.current_progress = 1.0;
                                ctx.request_repaint();
                            }
                        }
                    }
                }
            });
        }
    }

    enum OperationType {
        Split,
        Restore,
    }

    impl FileSplitterApp {
        fn start_operation(&mut self, ctx: egui::Context, op_type: OperationType) {
            self.is_processing = true;
            self.status_message = "Preparing...".to_string();
            self.current_progress = 0.0;

            let (tx, rx) = mpsc::channel();
            self.tx = Some(tx);
            self.rx = Some(rx);

            let tx_clone = self.tx.as_ref().unwrap().clone();
            let split_files_input_clone = self.split_files_input.clone();
            let split_size_limit_clone = self.split_size_limit.clone();
            let split_output_dir_clone = self.split_output_dir.clone();
            let split_compress_clone = self.split_compress;

            let restore_info_files_input_clone = self.restore_info_files_input.clone();
            let restore_input_dir_clone = self.restore_input_dir.clone();
            let restore_output_dir_clone = self.restore_output_dir.clone();

            thread::spawn(move || {
                let result: anyhow::Result<()> = match op_type {
                    OperationType::Split => {
                        let files: Vec<PathBuf> = split_files_input_clone.split(',')
                            .filter(|s| !s.trim().is_empty())
                            .map(|s| PathBuf::from(s.trim()))
                            .collect();
                        let size_limit = match split_size_limit_clone.parse::<u64>() {
                            Ok(s) => s,
                            Err(e) => return tx_clone.send(WorkerMessage::Error(format!("Invalid size limit: {}", e))).unwrap(),
                        };
                        let output_dir = PathBuf::from(split_output_dir_clone);

                        if files.is_empty() {
                            return tx_clone.send(WorkerMessage::Error("Please select files to split.".to_string())).unwrap();
                        }
                        if output_dir.to_str().unwrap_or("").is_empty() {
                            return tx_clone.send(WorkerMessage::Error("Please select an output directory.".to_string())).unwrap();
                        }

                        if let Err(e) = fs::create_dir_all(&output_dir) {
                            return tx_clone.send(WorkerMessage::Error(format!("Failed to create output directory: {}", e))).unwrap();
                        }

                        for file_path in files {
                            let ctx_for_progress = ctx.clone();
                            let ctx_for_message = ctx.clone();

                            let tx_progress = tx_clone.clone();
                            let tx_message = tx_clone.clone();
                            let progress_cb = Box::new(move |current, total| {
                                tx_progress.send(WorkerMessage::Progress(current, total)).unwrap();
                                ctx_for_progress.request_repaint();
                            });
                            let message_cb = Box::new(move |msg: String| {
                                tx_message.send(WorkerMessage::Message(msg)).unwrap();
                                ctx_for_message.request_repaint();
                            });

                            if let Err(e) = split_single_file(
                                &file_path,
                                size_limit,
                                &output_dir,
                                split_compress_clone,
                                Some(progress_cb),
                                Some(message_cb),
                            ) {
                                return tx_clone.send(WorkerMessage::Error(format!("File splitting failed: {}", e))).unwrap();
                            }
                        }
                        Ok(())
                    },
                    OperationType::Restore => {
                        let info_files: Vec<PathBuf> = restore_info_files_input_clone.split(',')
                            .filter(|s| !s.trim().is_empty())
                            .map(|s| PathBuf::from(s.trim()))
                            .collect();
                        let input_dir = PathBuf::from(restore_input_dir_clone);
                        let output_dir = PathBuf::from(restore_output_dir_clone);

                        if info_files.is_empty() {
                            return tx_clone.send(WorkerMessage::Error("Please select JSON info files to restore.".to_string())).unwrap();
                        }
                        if input_dir.to_str().unwrap_or("").is_empty() {
                            return tx_clone.send(WorkerMessage::Error("Please select the sub-files directory.".to_string())).unwrap();
                        }
                        if output_dir.to_str().unwrap_or("").is_empty() {
                            return tx_clone.send(WorkerMessage::Error("Please select the restoration output directory.".to_string())).unwrap();
                        }

                        if let Err(e) = fs::create_dir_all(&output_dir) {
                            return tx_clone.send(WorkerMessage::Error(format!("Failed to create output directory: {}", e))).unwrap();
                        }

                        for info_file_path in info_files {
                            let metadata_content = match fs::read_to_string(&info_file_path) {
                                Ok(content) => content,
                                Err(e) => return tx_clone.send(WorkerMessage::Error(format!("Failed to read restore info file {}: {}", info_file_path.display(), e))).unwrap(),
                            };
                            
                            let file_info: SplitInfo = match serde_json::from_str(&metadata_content) {
                                Ok(info) => info,
                                Err(e) => return tx_clone.send(WorkerMessage::Error(format!("Failed to parse restore info JSON file {}: {}", info_file_path.display(), e))).unwrap(),
                            };

                            let ctx_for_progress = ctx.clone();
                            let ctx_for_message = ctx.clone();

                            let tx_progress = tx_clone.clone();
                            let tx_message = tx_clone.clone();
                            let progress_cb = Box::new(move |current, total| {
                                tx_progress.send(WorkerMessage::Progress(current, total)).unwrap();
                                ctx_for_progress.request_repaint();
                            });
                            let message_cb = Box::new(move |msg: String| {
                                tx_message.send(WorkerMessage::Message(msg)).unwrap();
                                ctx_for_message.request_repaint();
                            });

                            if let Err(e) = restore_single_file(
                                &file_info,
                                &input_dir,
                                &output_dir,
                                Some(progress_cb),
                                Some(message_cb),
                            ) {
                                return tx_clone.send(WorkerMessage::Error(format!("File restoration failed: {}", e))).unwrap();
                            }
                        }
                        Ok(())
                    },
                };

                if result.is_ok() {
                    tx_clone.send(WorkerMessage::Done).unwrap();
                }
            });
        }
    }

    pub fn run_gui() -> Result<()> {
        let options = NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([660.0, 420.0]).with_resizable(false), // 设置窗口大小为660x420
            ..Default::default()
        };
        eframe::run_native(
            "File Splitter and Restorer",
            options,
            Box::new(|cc| {
                let mut fonts = egui::FontDefinitions::default();
                // 动态读取支持中文的字体 "Microsoft YaHei"
                let font_path = "C:\\Windows\\Fonts\\msyh.ttc";
                let font_bytes = fs::read(font_path).map_err(|e| anyhow::anyhow!("Failed to read font file {}: {}", font_path, e))?;
                let font_data = egui::FontData::from_owned(font_bytes);
                fonts.font_data.insert("msyh".to_owned(), font_data);
                fonts.families
                    .entry(egui::FontFamily::Proportional)
                    .or_insert_with(Vec::new)
                    .insert(0, "msyh".to_owned());
                cc.egui_ctx.set_fonts(fonts);
                Ok(Box::new(FileSplitterApp::default()))
            }),
        ).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    }
}

// --- Main function entry point ---
fn main() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        gui::run_gui()
    }

    #[cfg(not(target_os = "windows"))]
    {
        cli::run_cli()
    }
}