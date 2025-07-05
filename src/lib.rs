//! PKLib - Rust implementation of PKWare Data Compression Library
//!
//! This crate provides a pure Rust implementation of the PKWare DCL format (1980s DOS era),
//! compatible with the original PKLib by Ladislav Zezula. This format uses Huffman coding
//! and sliding dictionary compression, and is used in game archives like MPQ and other legacy applications.
//!
//! # Features
//!
//! - ✅ **Decompression (explode)** - Full PKLib compatibility verified
//! - ✅ **Compression (implode)** - PKLib-compatible compression
//! - Binary and ASCII compression modes
//! - Dictionary sizes: 1KB, 2KB, and 4KB
//! - Maximum repetition length: 516 bytes
//! - Streaming API via Read/Write traits
//! - Zero-copy where possible
//!
//! # Example - Decompression (Available Now)
//!
//! ```no_run
//! use pklib::{explode_bytes, ExplodeReader};
//! use std::io::Read;
//!
//! // Decompress PKLib-compressed data
//! let compressed_data = std::fs::read("data.imploded")?;
//! let decompressed = explode_bytes(&compressed_data)?;
//!
//! // Or use streaming API
//! let mut reader = ExplodeReader::new(std::io::Cursor::new(compressed_data))?;
//! let mut output = Vec::new();
//! reader.read_to_end(&mut output)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Example - Compression
//!
//! ```no_run
//! use pklib::{CompressionMode, DictionarySize, implode_bytes, ImplodeWriter};
//! use std::io::Write;
//!
//! // Compress data in-memory
//! let data = b"Hello, World! This is a test.";
//! let compressed = implode_bytes(data, CompressionMode::ASCII, DictionarySize::Size2K)?;
//!
//! // Or use streaming API
//! let mut output = Vec::new();
//! let mut writer = ImplodeWriter::new(&mut output, CompressionMode::ASCII, DictionarySize::Size2K)?;
//! writer.write_all(data)?;
//! let output = writer.finish()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

// Public modules
pub mod common;
pub mod crc32;
pub mod error;
pub mod explode;
pub mod implode;
pub mod tables;

// Async modules (only available with async feature)
#[cfg(feature = "async")]
pub mod async_batch;
#[cfg(feature = "async")]
pub mod async_convenience;
#[cfg(feature = "async")]
pub mod async_explode;
#[cfg(feature = "async")]
pub mod async_implode;
#[cfg(feature = "async")]
pub mod async_stream;

// Re-export commonly used types
pub use common::{
    CompressionHeader, CompressionMode, CompressionStats, DictionarySize, PkLibError, Result,
    MAX_REP_LENGTH, MAX_WINDOW_SIZE,
};
pub use crc32::{crc32, crc32_pklib};
pub use explode::{explode_mpq_bytes, ExplodeReader};
pub use implode::ImplodeWriter;

// Re-export async types when async feature is enabled
#[cfg(feature = "async")]
pub use async_batch::AsyncBatchProcessor;
#[cfg(feature = "async")]
pub use async_convenience::*;
#[cfg(feature = "async")]
pub use async_explode::AsyncExplodeReader;
#[cfg(feature = "async")]
pub use async_implode::AsyncImplodeWriter;
#[cfg(feature = "async")]
pub use async_stream::{AsyncStreamProcessor, StreamOptions};

// Convenience functions

/// Compress data using the PKWare implode algorithm
///
/// # Arguments
/// * `data` - The data to compress
/// * `mode` - Compression mode (Binary or ASCII)
/// * `dict_size` - Dictionary size (1KB, 2KB, or 4KB)
///
/// # Returns
/// A vector containing the compressed data
pub fn implode_bytes(
    data: &[u8],
    mode: CompressionMode,
    dict_size: DictionarySize,
) -> Result<Vec<u8>> {
    implode::implode_bytes(data, mode, dict_size)
}

/// Decompress data using the PKWare explode algorithm
///
/// # Arguments
/// * `data` - The compressed data
///
/// # Returns
/// A vector containing the decompressed data
pub fn explode_bytes(data: &[u8]) -> Result<Vec<u8>> {
    explode::explode_bytes(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reexports() {
        // Test that common types are accessible
        let _ = CompressionMode::Binary;
        let _ = DictionarySize::Size2K;

        // Test that functions are accessible
        let data = b"test";
        let _ = crc32(data);
    }
}
