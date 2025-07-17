// src/lib.rs
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path; // Removed PathBuf from import
use anyhow::{Result, Context};
use sha2::{Sha256, Digest};
use hex;
use flate2::{
    write::GzEncoder,
    read::GzDecoder,
    Compression,
};

/// Information for a single chunk after file splitting
#[derive(Serialize, Deserialize, Debug, Clone)] // Added Clone for GUI state management
pub struct ChunkInfo {
    /// Filename of the chunk (e.g., "my_file-001")
    pub chunk_filename: String,
    /// Actual size of this chunk in bytes (if compressed, this is the compressed size)
    pub chunk_size: u64,
    /// SHA256 checksum of the original (uncompressed) content of this chunk (optional, for finer-grained verification)
    pub chunk_checksum: Option<String>,
}

/// Split information for an original file
#[derive(Serialize, Deserialize, Debug, Clone)] // Added Clone for GUI state management
pub struct SplitInfo {
    /// Original filename
    pub original_filename: String,
    /// Total size of the original file in bytes
    pub original_file_size: u64,
    /// Maximum size limit set for each chunk during splitting (bytes)
    pub chunk_limit: u64,
    /// Name of the subdirectory containing all chunks for this file (e.g., "my_file_parts")
    pub chunks_sub_dir: String,
    /// Detailed list of all chunks
    pub chunks: Vec<ChunkInfo>,
    /// SHA256 checksum of the original file
    pub original_checksum: String,
    /// Whether the split sub-files were compressed
    pub is_compressed: bool,
}

/// Splits a single file or copies it (if no splitting is needed)
///
/// `file_path`: Path to the file to split.
/// `size_limit`: Maximum size limit for each chunk in bytes.
/// `output_root_dir`: Root directory where split sub-files and info files will be stored.
/// `compress`: Whether to Gzip compress the split sub-files.
/// `progress_callback`: Optional callback for reporting progress (current_bytes, total_bytes).
/// `message_callback`: Optional callback for reporting messages (message string).
pub fn split_single_file(
    file_path: &Path,
    size_limit: u64,
    output_root_dir: &Path,
    compress: bool,
    progress_callback: Option<Box<dyn Fn(u64, u64) + Send + Sync + 'static>>,
    message_callback: Option<Box<dyn Fn(String) + Send + Sync + 'static>>,
) -> Result<()> {
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
    
    let metadata = file.metadata()?;
    let original_file_size = metadata.len();
    let filename_str = file_path.file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename: {}", file_path.display()))?
        .to_string();

    // Create a dedicated subdirectory for this file's chunks
    let chunks_sub_dir_name = format!("{}_parts", filename_str);
    let chunks_output_dir = output_root_dir.join(&chunks_sub_dir_name); // This returns PathBuf
    fs::create_dir_all(&chunks_output_dir)
        .with_context(|| format!("Failed to create subdirectory: {}", chunks_output_dir.display()))?;

    let original_checksum = calculate_checksum(file_path)?;

    let mut reader = BufReader::new(file);
    let mut chunk_index = 0; // Starts from 001
    let mut chunks_info = Vec::new();
    let mut total_bytes_processed = 0u64;

    if let Some(cb) = &message_callback {
        cb(format!("Splitting '{}'", filename_str));
    }

    loop {
        chunk_index += 1;
        let chunk_filename = format!("{}-{:03}", filename_str, chunk_index);
        let chunk_path = chunks_output_dir.join(&chunk_filename); // This returns PathBuf
        
        let mut buffer = vec![0u8; size_limit as usize]; // Use size_limit as buffer size
        let bytes_read = reader.read(&mut buffer)?; // Read original data
        
        if bytes_read == 0 {
            // If the file size is less than or equal to size_limit, and this is the only read, then only one chunk is generated.
            // But if the file is empty, it will break here directly, and chunks_info will be empty, which needs to be handled.
            if chunks_info.is_empty() && original_file_size == 0 {
                // Handle empty file case
                chunks_info.push(ChunkInfo {
                    chunk_filename: format!("{}-001", filename_str), // Even for empty files, give a chunk name
                    chunk_size: 0,
                    chunk_checksum: Some(calculate_buffer_checksum(&[])), // Checksum for empty file
                });
            }
            break;
        }
        
        let original_chunk_data = &buffer[..bytes_read];
        let original_chunk_checksum = Some(calculate_buffer_checksum(original_chunk_data));

        let mut file_writer = File::create(&chunk_path)
            .with_context(|| format!("Failed to create chunk file: {}", chunk_path.display()))?;

        let actual_chunk_size;

        if compress {
            let mut encoder = GzEncoder::new(file_writer, Compression::default());
            encoder.write_all(original_chunk_data)?;
            actual_chunk_size = encoder.finish()?.metadata()?.len(); // Get compressed file size
        } else {
            file_writer.write_all(original_chunk_data)?;
            file_writer.flush()?;
            actual_chunk_size = original_chunk_data.len() as u64; // Uncompressed, directly the original data size
        }
        
        chunks_info.push(ChunkInfo {
            chunk_filename,
            chunk_size: actual_chunk_size, // Record actual size (compressed or uncompressed)
            chunk_checksum: original_chunk_checksum, // Record checksum of original (uncompressed) data
        });
        total_bytes_processed += bytes_read as u64; // Total bytes processed is still the sum of original file bytes
        
        if let Some(cb) = &progress_callback {
            cb(total_bytes_processed, original_file_size);
        }

        // If the number of bytes read is less than size_limit, it means it's the last part of the file
        if (bytes_read as u64) < size_limit {
            break;
        }
    }
    
    if let Some(cb) = &message_callback {
        cb(format!("'{}' splitting complete", filename_str));
    }

    // Verify total size matches
    if total_bytes_processed != original_file_size {
        return Err(anyhow::anyhow!(
            "File size mismatch during splitting: Expected {}, Actual {}",
            original_file_size,
            total_bytes_processed
        ));
    }

    // Build SplitInfo
    let split_info = SplitInfo {
        original_filename: filename_str.clone(),
        original_file_size,
        chunk_limit: size_limit,
        chunks_sub_dir: chunks_sub_dir_name,
        chunks: chunks_info,
        original_checksum,
        is_compressed: compress, // Record whether compressed
    };

    // Save SplitInfo to JSON file
    let info_filename = format!("{}.json", filename_str);
    let info_path = chunks_output_dir.join(&info_filename); // This returns PathBuf
    let json_data = serde_json::to_string_pretty(&split_info)?;
    fs::write(&info_path, json_data)
        .with_context(|| format!("Failed to save split info JSON file: {}", info_path.display()))?;
    
    if let Some(cb) = &message_callback {
        cb(format!("Split info for file '{}' saved to: {}", filename_str, info_path.display()));
    }

    Ok(())
}

/// Restores a single file
///
/// `file_info`: Split information for the file to restore.
/// `input_root_dir`: Root directory where the split sub-files are located.
/// `output_dir`: Directory where the restored large file will be saved.
/// `progress_callback`: Optional callback for reporting progress (current_bytes, total_bytes).
/// `message_callback`: Optional callback for reporting messages (message string).
pub fn restore_single_file(
    file_info: &SplitInfo,
    input_root_dir: &Path,
    output_dir: &Path,
    progress_callback: Option<Box<dyn Fn(u64, u64) + Send + Sync + 'static>>,
    message_callback: Option<Box<dyn Fn(String) + Send + Sync + 'static>>,
) -> Result<()> {
    let output_path = output_dir.join(&file_info.original_filename); // This returns PathBuf
    let mut output_file = BufWriter::new(File::create(&output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?);
    
    if let Some(cb) = &message_callback {
        cb(format!("Restoring '{}'", file_info.original_filename));
    }

    let mut total_written = 0u64;

    // Locate the subdirectory containing chunks for the current file
    let chunks_input_dir = input_root_dir.join(&file_info.chunks_sub_dir); // This returns PathBuf
    if !chunks_input_dir.exists() {
        return Err(anyhow::anyhow!(
            "Chunk directory for file '{}' not found: {}",
            file_info.original_filename,
            chunks_input_dir.display()
        ));
    }

    for chunk_info in &file_info.chunks {
        let chunk_path = chunks_input_dir.join(&chunk_info.chunk_filename); // This returns PathBuf
        let chunk_file = File::open(&chunk_path)
            .with_context(|| format!("Failed to open chunk file: {}", chunk_path.display()))?;
        
        let mut decompressed_data = Vec::new();
        let bytes_read_current_chunk_decompressed;

        if file_info.is_compressed {
            let mut decoder = GzDecoder::new(chunk_file);
            bytes_read_current_chunk_decompressed = decoder.read_to_end(&mut decompressed_data)?;
        } else {
            let mut reader = BufReader::new(chunk_file);
            bytes_read_current_chunk_decompressed = reader.read_to_end(&mut decompressed_data)?;
        }
        
        // Verify checksum of the original (uncompressed) chunk data (if available)
        if let Some(expected_checksum) = &chunk_info.chunk_checksum {
            let actual_checksum = calculate_buffer_checksum(&decompressed_data[..bytes_read_current_chunk_decompressed]);
            if actual_checksum != *expected_checksum {
                eprintln!("Warning: Checksum mismatch for chunk '{}'! Expected: {}, Actual: {}", 
                          chunk_info.chunk_filename, expected_checksum, actual_checksum);
                // You can choose to return an error here, or continue, depending on data integrity requirements
            }
        }

        output_file.write_all(&decompressed_data[..bytes_read_current_chunk_decompressed])?;
        total_written += bytes_read_current_chunk_decompressed as u64;
        
        if let Some(cb) = &progress_callback {
            cb(total_written, file_info.original_file_size);
        }
    }
    
    output_file.flush()?;
    if let Some(cb) = &message_callback {
        cb(format!("'{}' restoration complete", file_info.original_filename));
    }

    // Verify restored file size
    let restored_file = File::open(&output_path)?;
    let restored_size = restored_file.metadata()?.len();
    if restored_size != file_info.original_file_size {
        return Err(anyhow::anyhow!(
            "Restored file size mismatch: Expected {}, Actual {}",
            file_info.original_file_size,
            restored_size
        ));
    }

    // Verify original file checksum
    let actual_original_checksum = calculate_checksum(&output_path)?;
    if actual_original_checksum != file_info.original_checksum {
        eprintln!("Warning: Original checksum mismatch for restored file '{}'! Expected: {}, Actual: {}", 
                  file_info.original_filename, file_info.original_checksum, actual_original_checksum);
        // You can choose to return an error here
    }

    Ok(())
}

/// Calculates the SHA256 checksum of file content
pub fn calculate_checksum(file_path: &Path) -> Result<String> {
    let mut file = File::open(file_path)
        .with_context(|| format!("Failed to open file to calculate checksum: {}", file_path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192]; // 8KB buffer
    
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(hex::encode(hasher.finalize()))
}

/// Calculates the SHA256 checksum of buffer content
pub fn calculate_buffer_checksum(buffer: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(buffer);
    hex::encode(hasher.finalize())
}

