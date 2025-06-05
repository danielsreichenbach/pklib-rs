//! blast-cli - Command-line interface for PKLib
//!
//! A command-line tool for compressing and decompressing files using the PKWare DCL format.

use clap::{Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use pklib::{explode_bytes, implode_bytes, CompressionMode, DictionarySize};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "blast-cli")]
#[command(about = "A CLI tool for PKWare DCL compression and decompression")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Quiet mode (suppress non-error output)
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Compress a file using PKLib format
    Compress {
        /// Input file to compress
        input: PathBuf,

        /// Output compressed file
        output: PathBuf,

        /// Compression mode
        #[arg(short, long, value_enum, default_value_t = CliCompressionMode::Binary)]
        mode: CliCompressionMode,

        /// Dictionary size
        #[arg(short, long, value_enum, default_value_t = CliDictionarySize::Size2K)]
        dict_size: CliDictionarySize,

        /// Force overwrite of output file
        #[arg(short, long)]
        force: bool,
    },

    /// Decompress a PKLib-compressed file
    Decompress {
        /// Input compressed file
        input: PathBuf,

        /// Output decompressed file
        output: PathBuf,

        /// Force overwrite of output file
        #[arg(short, long)]
        force: bool,
    },

    /// Get information about a compressed file
    Info {
        /// Compressed file to analyze
        input: PathBuf,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CliCompressionMode {
    /// Binary compression mode (optimized for binary data)
    Binary,
    /// ASCII compression mode (optimized for text data)
    Ascii,
}

impl From<CliCompressionMode> for CompressionMode {
    fn from(mode: CliCompressionMode) -> Self {
        match mode {
            CliCompressionMode::Binary => CompressionMode::Binary,
            CliCompressionMode::Ascii => CompressionMode::ASCII,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CliDictionarySize {
    /// 1KB dictionary (1024 bytes)
    Size1K,
    /// 2KB dictionary (2048 bytes) - Default
    Size2K,
    /// 4KB dictionary (4096 bytes)
    Size4K,
}

impl From<CliDictionarySize> for DictionarySize {
    fn from(size: CliDictionarySize) -> Self {
        match size {
            CliDictionarySize::Size1K => DictionarySize::Size1K,
            CliDictionarySize::Size2K => DictionarySize::Size2K,
            CliDictionarySize::Size4K => DictionarySize::Size4K,
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Compress {
            input,
            output,
            mode,
            dict_size,
            force,
        } => compress_file(
            &input,
            &output,
            mode.into(),
            dict_size.into(),
            force,
            cli.verbose,
            cli.quiet,
        ),
        Commands::Decompress {
            input,
            output,
            force,
        } => decompress_file(&input, &output, force, cli.verbose, cli.quiet),
        Commands::Info { input } => show_file_info(&input, cli.verbose),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn compress_file(
    input: &PathBuf,
    output: &PathBuf,
    mode: CompressionMode,
    dict_size: DictionarySize,
    force: bool,
    verbose: bool,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if input file exists
    if !input.exists() {
        return Err(format!("Input file '{}' does not exist", input.display()).into());
    }

    // Check if output file exists and force flag
    if output.exists() && !force {
        return Err(format!(
            "Output file '{}' already exists. Use --force to overwrite",
            output.display()
        )
        .into());
    }

    if verbose {
        println!(
            "Compressing '{}' to '{}'",
            input.display(),
            output.display()
        );
        println!("Mode: {:?}, Dictionary: {:?}", mode, dict_size);
    }

    let start_time = Instant::now();

    // Read input file
    let input_data = fs::read(input)?;
    let input_size = input_data.len();

    if verbose {
        println!("Input size: {} bytes", input_size);
    }

    // Show progress bar for large files
    let progress = if !quiet && input_size > 1024 * 1024 {
        let pb = ProgressBar::new(2);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message("Compressing...");
        Some(pb)
    } else {
        None
    };

    if let Some(ref pb) = progress {
        pb.inc(1);
    }

    // Compress data
    let compressed_data = implode_bytes(&input_data, mode, dict_size)
        .map_err(|e| format!("Compression failed: {}", e))?;

    if let Some(ref pb) = progress {
        pb.inc(1);
        pb.finish_with_message("Compression complete");
    }

    // Write output file
    fs::write(output, &compressed_data)?;

    let compression_time = start_time.elapsed();
    let output_size = compressed_data.len();
    let compression_ratio = (output_size as f64 / input_size as f64) * 100.0;

    if !quiet {
        println!("✓ Compression successful!");
        println!("  Input:  {} bytes", input_size);
        println!("  Output: {} bytes", output_size);
        println!("  Ratio:  {:.1}%", compression_ratio);
        println!("  Time:   {:.2?}", compression_time);

        if compression_ratio > 100.0 {
            println!("  Note: File expanded during compression (common for small/random data)");
        }
    }

    Ok(())
}

fn decompress_file(
    input: &PathBuf,
    output: &PathBuf,
    force: bool,
    verbose: bool,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if input file exists
    if !input.exists() {
        return Err(format!("Input file '{}' does not exist", input.display()).into());
    }

    // Check if output file exists and force flag
    if output.exists() && !force {
        return Err(format!(
            "Output file '{}' already exists. Use --force to overwrite",
            output.display()
        )
        .into());
    }

    if verbose {
        println!(
            "Decompressing '{}' to '{}'",
            input.display(),
            output.display()
        );
    }

    let start_time = Instant::now();

    // Read input file
    let compressed_data = fs::read(input)?;
    let input_size = compressed_data.len();

    if verbose {
        println!("Compressed size: {} bytes", input_size);
    }

    // Show progress bar for large files
    let progress = if !quiet && input_size > 1024 * 1024 {
        let pb = ProgressBar::new(2);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message("Decompressing...");
        Some(pb)
    } else {
        None
    };

    if let Some(ref pb) = progress {
        pb.inc(1);
    }

    // Decompress data
    let decompressed_data =
        explode_bytes(&compressed_data).map_err(|e| format!("Decompression failed: {}", e))?;

    if let Some(ref pb) = progress {
        pb.inc(1);
        pb.finish_with_message("Decompression complete");
    }

    // Write output file
    fs::write(output, &decompressed_data)?;

    let decompression_time = start_time.elapsed();
    let output_size = decompressed_data.len();
    let compression_ratio = (input_size as f64 / output_size as f64) * 100.0;

    if !quiet {
        println!("✓ Decompression successful!");
        println!("  Input:  {} bytes", input_size);
        println!("  Output: {} bytes", output_size);
        println!("  Ratio:  {:.1}%", compression_ratio);
        println!("  Time:   {:.2?}", decompression_time);
    }

    Ok(())
}

fn show_file_info(input: &PathBuf, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Check if input file exists
    if !input.exists() {
        return Err(format!("Input file '{}' does not exist", input.display()).into());
    }

    // Read the file
    let data = fs::read(input)?;
    let file_size = data.len();

    if data.len() < 3 {
        return Err("File too small to be a valid PKLib compressed file".into());
    }

    // Parse PKLib header
    let compression_type = data[0];
    let dict_bits = data[1];

    let mode_str = match compression_type {
        0 => "Binary",
        1 => "ASCII",
        _ => "Unknown",
    };

    let dict_size_str = match dict_bits {
        4 => "1KB (1024 bytes)",
        5 => "2KB (2048 bytes)",
        6 => "4KB (4096 bytes)",
        _ => "Unknown",
    };

    println!("PKLib File Information:");
    println!("  File: {}", input.display());
    println!("  Size: {} bytes", file_size);
    println!("  Compression Mode: {} ({})", mode_str, compression_type);
    println!("  Dictionary Size: {} ({} bits)", dict_size_str, dict_bits);

    if verbose {
        println!(
            "  Header bytes: {:02x} {:02x} {:02x}",
            data[0], data[1], data[2]
        );
    }

    // Try to get decompressed size by attempting decompression
    match explode_bytes(&data) {
        Ok(decompressed) => {
            let decompressed_size = decompressed.len();
            let compression_ratio = (file_size as f64 / decompressed_size as f64) * 100.0;
            println!("  Decompressed Size: {} bytes", decompressed_size);
            println!("  Compression Ratio: {:.1}%", compression_ratio);
            println!("  Status: ✓ Valid PKLib file");
        }
        Err(e) => {
            println!("  Status: ✗ Invalid or corrupted PKLib file");
            if verbose {
                println!("  Error: {}", e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let input_path = dir.path().join("input.txt");
        let compressed_path = dir.path().join("compressed.pklib");
        let output_path = dir.path().join("output.txt");

        // Create test data
        let test_data = b"Hello, World! This is a test of the PKLib CLI tool.";
        fs::write(&input_path, test_data)?;

        // Compress
        compress_file(
            &input_path,
            &compressed_path,
            CompressionMode::ASCII,
            DictionarySize::Size2K,
            false,
            false,
            true,
        )?;

        // Decompress
        decompress_file(&compressed_path, &output_path, false, false, true)?;

        // Verify
        let result_data = fs::read(&output_path)?;
        assert_eq!(test_data, &result_data[..]);

        Ok(())
    }
}
