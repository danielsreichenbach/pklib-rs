# pklib

A pure Rust implementation of the PKWare Data Compression Library (DCL), providing high-performance compression and decompression compatible with the original PKLib by Ladislav Zezula.

[![Crates.io](https://img.shields.io/crates/v/pklib.svg)](https://crates.io/crates/pklib)
[![Documentation](https://docs.rs/pklib/badge.svg)](https://docs.rs/pklib)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

PKLib implements the PKWare DCL format used in many legacy applications and games. It provides both compression ("implode") and decompression ("explode") functionality with full compatibility to the original PKLib specification.

### Key Features

- 🔄 **Full PKLib Compatibility** - Bit-for-bit compatible with original PKLib
- 🚀 **High Performance** - Optimized Rust implementation with zero-copy where possible
- 🛡️ **Memory Safe** - Written in safe Rust with comprehensive error handling
- 📦 **Multiple Formats** - Support for Binary and ASCII compression modes
- 🎯 **Flexible Dictionary Sizes** - 1KB, 2KB, and 4KB dictionary support
- 📏 **Extended Length Support** - Maximum repetition length of 516 bytes
- 🔌 **Streaming API** - Implements standard `Read`/`Write` traits
- 📚 **Well Documented** - Comprehensive documentation and examples

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
pklib = "0.1"
```

### Basic Usage

```rust
use pklib::{CompressionMode, DictionarySize, explode_bytes, implode_bytes};

// Decompress PKLib-compressed data
let compressed_data = std::fs::read("data.imploded")?;
let decompressed = explode_bytes(&compressed_data)?;

// Compress data using PKLib format
let data = b"Hello, World! This is a test of the PKLib compression.";
let compressed = implode_bytes(data, CompressionMode::ASCII, DictionarySize::Size2K)?;
```

### Streaming API

```rust
use std::io::{Read, Write};
use pklib::{ExplodeReader, ImplodeWriter, CompressionMode, DictionarySize};

// Decompress using streaming API
let compressed_data = std::fs::read("large_file.imploded")?;
let mut decompressor = ExplodeReader::new(std::io::Cursor::new(compressed_data))?;

let mut decompressed = Vec::new();
decompressor.read_to_end(&mut decompressed)?;

// Compress using streaming API
let mut output = Vec::new();
let mut compressor = ImplodeWriter::new(&mut output, CompressionMode::Binary, DictionarySize::Size4K)?;
compressor.write_all(b"Data to compress")?;
let compressed_output = compressor.finish()?;
```

## Command Line Interface

pklib includes a powerful CLI tool called `blast-cli` for compressing and decompressing files:

### Installation

```bash
# Install from source
cargo install --path .

# Or run directly
cargo run --bin blast-cli -- --help
```

### CLI Usage

#### Compress a file

```bash
# Basic compression with ASCII mode (good for text)
blast-cli compress input.txt output.pklib --mode ascii

# Binary mode with 4KB dictionary (good for binary data)
blast-cli compress data.bin compressed.pklib --mode binary --dict-size size4-k

# Force overwrite existing files
blast-cli compress input.txt output.pklib --force
```

#### Decompress a file

```bash
# Basic decompression
blast-cli decompress compressed.pklib restored.txt

# With verbose output
blast-cli --verbose decompress compressed.pklib restored.txt
```

#### Analyze compressed files

```bash
# Get information about a compressed file
blast-cli info compressed.pklib

# Verbose output shows additional details
blast-cli --verbose info compressed.pklib
```

### CLI Options

- `--mode`: Choose `binary` (default) or `ascii` compression mode
- `--dict-size`: Dictionary size - `size1-k`, `size2-k` (default), or `size4-k`
- `--force`: Overwrite existing output files
- `--verbose`: Show detailed progress and statistics
- `--quiet`: Suppress non-error output

## Compression Modes

PKLib supports two compression modes optimized for different data types:

- **Binary Mode** - Optimized for binary data (executables, images, etc.)
- **ASCII Mode** - Optimized for text data with better compression ratios

## Dictionary Sizes

Choose the dictionary size based on your data characteristics:

- **1KB (1024 bytes)** - Fastest compression, smaller memory usage
- **2KB (2048 bytes)** - Balanced performance and compression ratio
- **4KB (4096 bytes)** - Best compression ratio, higher memory usage

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Core Infrastructure | ✅ Complete | Types, errors, constants |
| Static Lookup Tables | ✅ Complete | All PKLib tables ported |
| CRC32 Implementation | ✅ Complete | PKLib-compatible checksums |
| Decompression (Explode) | ✅ Complete | Full PKLib compatibility verified |
| Compression (Implode) | ✅ Complete | Full PKLib compatibility verified |
| Testing & Validation | ✅ Complete | Comprehensive test suite with property testing |
| CLI Tool | ✅ Complete | Command-line interface with compress/decompress/info commands |

## Performance

PKLib is designed for high performance with several optimizations:

- Compile-time lookup table generation
- Efficient bit manipulation routines
- Zero-copy operations where possible
- Minimal memory allocations in hot paths
- Streaming API for processing large files without loading into memory

Performance characteristics:

- **Decompression**: Fast single-pass algorithm with bit-level decoding
- **Compression**: Hash-based pattern matching with 4-tier length encoding
- **Memory Usage**: Configurable dictionary sizes (1KB-4KB) for different memory constraints
- **Throughput**: Competitive with original PKLib C implementation

### Benchmarks

PKLib includes a comprehensive benchmark suite to measure performance across various scenarios:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench compression
cargo bench decompression
cargo bench round_trip
cargo bench memory
cargo bench concurrent

# Save baseline for comparison
cargo bench -- --save-baseline initial

# Compare against baseline
cargo bench -- --baseline initial
```

The benchmark suite covers:

- **Throughput**: MB/s for different file sizes and data patterns
- **Compression Ratios**: Effectiveness across various data types
- **Memory Usage**: Peak allocation tracking with custom allocator
- **Round-trip Performance**: Complete compress/decompress cycles
- **Concurrent Processing**: Multi-threaded performance scaling

## Compatibility

This implementation achieves 100% compatibility with:

- ✅ **Original PKLib** by Ladislav Zezula (StormLib) - Full bit-for-bit compatibility verified
- ✅ **PKWare DCL Format** - Complete specification implementation with all edge cases
- ✅ **Legacy Applications** - Successfully processes files from games and archived software
- ✅ **Round-trip Testing** - Compression/decompression cycles preserve data integrity
- ✅ **All Compression Modes** - Binary and ASCII modes with 1KB/2KB/4KB dictionaries

## Contributing

Contributions are welcome!

### Development Setup

```bash
# Clone the repository
git clone https://github.com/danielsreichenbach/pklib-rs.git
cd pklib-rs

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy --all-targets --all-features

# Run benchmarks
cargo bench
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **Ladislav Zezula** - Original PKLib implementation
- **PKWare Inc.** - Original DCL format specification
- **StormLib Project** - Reference implementation and test cases

## References

- [PKLib Reference Implementation](https://codeberg.org/implode-compression-impls/pklib)
- [PKWare DCL Format Documentation](https://wiki.multimedia.cx/index.php/PKWare_DCL)

---

**Status**: All 4 implementation phases are complete! PKLib provides a fully functional, production-ready implementation of the PKWare DCL format with comprehensive testing and validation.
