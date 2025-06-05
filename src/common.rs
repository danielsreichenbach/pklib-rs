//! Common types and constants for PKWare Data Compression Library
//!
//! This module defines the core types, constants, and structures used by both
//! the compression (implode) and decompression (explode) algorithms.

use thiserror::Error;

/// Compression mode for the PKWare DCL format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMode {
    /// Binary mode - optimized for binary data
    Binary = 0,
    /// ASCII mode - optimized for text data
    ASCII = 1,
}

impl CompressionMode {
    /// Create a CompressionMode from a raw value
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0 => Ok(CompressionMode::Binary),
            1 => Ok(CompressionMode::ASCII),
            _ => Err(PkLibError::InvalidCompressionMode(value)),
        }
    }
}

/// Dictionary size for compression/decompression
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionarySize {
    /// 1024 bytes (1KB) dictionary
    Size1K = 1024,
    /// 2048 bytes (2KB) dictionary
    Size2K = 2048,
    /// 4096 bytes (4KB) dictionary
    Size4K = 4096,
}

impl DictionarySize {
    /// Get the number of bits needed to represent this dictionary size
    pub fn bits(&self) -> u8 {
        match self {
            DictionarySize::Size1K => 4, // 2^10 = 1024, needs 10 bits, 10-6=4
            DictionarySize::Size2K => 5, // 2^11 = 2048, needs 11 bits, 11-6=5
            DictionarySize::Size4K => 6, // 2^12 = 4096, needs 12 bits, 12-6=6
        }
    }

    /// Get the bit mask for this dictionary size
    pub fn mask(&self) -> u32 {
        (*self as u32) - 1
    }

    /// Create a DictionarySize from the number of bits
    pub fn from_bits(bits: u8) -> Result<Self> {
        match bits {
            4 => Ok(DictionarySize::Size1K),
            5 => Ok(DictionarySize::Size2K),
            6 => Ok(DictionarySize::Size4K),
            _ => Err(PkLibError::InvalidDictionaryBits(bits)),
        }
    }

    /// Create a DictionarySize from byte size
    pub fn from_bytes(bytes: u32) -> Result<Self> {
        match bytes {
            1024 => Ok(DictionarySize::Size1K),
            2048 => Ok(DictionarySize::Size2K),
            4096 => Ok(DictionarySize::Size4K),
            _ => Err(PkLibError::InvalidDictionarySize(bytes)),
        }
    }
}

/// Error type for PKLib operations
#[derive(Debug, Error)]
pub enum PkLibError {
    /// Invalid compression mode value
    #[error("Invalid compression mode: {0}")]
    InvalidCompressionMode(u8),

    /// Invalid dictionary size bits
    #[error("Invalid dictionary bits: {0} (expected 4, 5, or 6)")]
    InvalidDictionaryBits(u8),

    /// Invalid dictionary size
    #[error("Invalid dictionary size: {0} (expected 1024, 2048, or 4096)")]
    InvalidDictionarySize(u32),

    /// Invalid compressed data format
    #[error("Invalid compressed data format")]
    InvalidFormat,

    /// Unexpected end of input
    #[error("Unexpected end of input")]
    UnexpectedEof,

    /// Output buffer too small
    #[error("Output buffer too small")]
    BufferTooSmall,

    /// Invalid length encoding
    #[error("Invalid length encoding: {0}")]
    InvalidLength(u32),

    /// Invalid distance encoding
    #[error("Invalid distance encoding: {0}")]
    InvalidDistance(u32),

    /// Invalid data format or corruption
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Decompression error
    #[error("Decompression error: {0}")]
    DecompressionError(String),

    /// CRC32 checksum mismatch
    #[error("CRC32 checksum mismatch: expected {expected:08X}, got {actual:08X}")]
    CrcMismatch {
        /// Expected CRC32 value
        expected: u32,
        /// Actual CRC32 value
        actual: u32,
    },

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type alias for PKLib operations
pub type Result<T> = std::result::Result<T, PkLibError>;

// PKLib-specific constants

/// Maximum repetition length (distance match length)
pub const MAX_REP_LENGTH: u32 = 0x204; // 516 bytes

/// Maximum window size for decompression
pub const MAX_WINDOW_SIZE: usize = 0x3000; // 12KB

/// Size of the compression work buffer
pub const WORK_BUFF_SIZE: usize = 0x2204;

/// Size of the compression output buffer
pub const OUT_BUFF_SIZE: usize = 0x802;

/// Size of the decompression input buffer
pub const IN_BUFF_SIZE: usize = 0x800;

/// Hash table size for compression
pub const HASH_TABLE_SIZE: usize = 0x900;

/// Minimum match length for compression
pub const MIN_MATCH_LENGTH: usize = 3;

/// PKLib file signature (if used)
pub const PKLIB_SIGNATURE: u32 = 0x00088B1F;

/// Compression header structure
#[derive(Debug, Clone, Copy)]
pub struct CompressionHeader {
    /// Compression mode (Binary/ASCII)
    pub mode: CompressionMode,
    /// Dictionary size
    pub dict_size: DictionarySize,
    /// Original uncompressed size (optional)
    pub uncompressed_size: Option<u32>,
    /// CRC32 of uncompressed data (optional)
    pub crc32: Option<u32>,
}

/// Statistics for compression/decompression operations
#[derive(Debug, Default, Clone)]
pub struct CompressionStats {
    /// Number of literal bytes encoded/decoded
    pub literal_count: usize,
    /// Number of distance matches encoded/decoded
    pub match_count: usize,
    /// Total bytes processed
    pub bytes_processed: usize,
    /// Longest match found
    pub longest_match: usize,
    /// Input bytes (for async operations)
    pub input_bytes: u64,
    /// Output bytes (for async operations)
    pub output_bytes: u64,
    /// Compression ratio (for async operations)
    pub compression_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_mode() {
        assert_eq!(
            CompressionMode::from_u8(0).unwrap(),
            CompressionMode::Binary
        );
        assert_eq!(CompressionMode::from_u8(1).unwrap(), CompressionMode::ASCII);
        assert!(CompressionMode::from_u8(2).is_err());
    }

    #[test]
    fn test_dictionary_size() {
        // Test bits
        assert_eq!(DictionarySize::Size1K.bits(), 4);
        assert_eq!(DictionarySize::Size2K.bits(), 5);
        assert_eq!(DictionarySize::Size4K.bits(), 6);

        // Test masks
        assert_eq!(DictionarySize::Size1K.mask(), 0x3FF);
        assert_eq!(DictionarySize::Size2K.mask(), 0x7FF);
        assert_eq!(DictionarySize::Size4K.mask(), 0xFFF);

        // Test from_bits
        assert_eq!(
            DictionarySize::from_bits(4).unwrap(),
            DictionarySize::Size1K
        );
        assert_eq!(
            DictionarySize::from_bits(5).unwrap(),
            DictionarySize::Size2K
        );
        assert_eq!(
            DictionarySize::from_bits(6).unwrap(),
            DictionarySize::Size4K
        );
        assert!(DictionarySize::from_bits(7).is_err());

        // Test from_bytes
        assert_eq!(
            DictionarySize::from_bytes(1024).unwrap(),
            DictionarySize::Size1K
        );
        assert_eq!(
            DictionarySize::from_bytes(2048).unwrap(),
            DictionarySize::Size2K
        );
        assert_eq!(
            DictionarySize::from_bytes(4096).unwrap(),
            DictionarySize::Size4K
        );
        assert!(DictionarySize::from_bytes(512).is_err());
    }

    #[test]
    fn test_constants() {
        assert_eq!(MAX_REP_LENGTH, 516);
        assert_eq!(MAX_WINDOW_SIZE, 0x3000);
        assert_eq!(WORK_BUFF_SIZE, 0x2204);
        assert_eq!(OUT_BUFF_SIZE, 0x802);
        assert_eq!(IN_BUFF_SIZE, 0x800);
        assert_eq!(HASH_TABLE_SIZE, 0x900);
    }
}
