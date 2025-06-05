//! PKLib Implode (compression) implementation
//!
//! This module provides compression functionality compatible with the PKWare DCL format.
//! It implements the implode algorithm exactly as specified in the original PKLib.

mod hash;
mod pattern;
mod state;
mod writer;

pub use state::ImplodeState;
pub use writer::ImplodeWriter;

use crate::Result;
use std::io::Write;

/// Work buffer size for compression (8708 bytes)
pub const WORK_BUFF_SIZE: usize = 0x2204;

/// Output buffer size for compression (2050 bytes)
pub const OUT_BUFF_SIZE: usize = 0x802;

/// Hash table size for pattern matching (2304 entries)
pub const HASH_TABLE_SIZE: usize = 0x900;

/// Offset table size for compression optimization (516 bytes)
pub const OFFSS_SIZE2: usize = 0x204;

/// Total number of literal codes including length codes (774)
pub const LITERALS_COUNT: usize = 0x306;

/// PKLib hash function for byte pairs
/// Formula: (byte0 * 4) + (byte1 * 5)
pub const fn byte_pair_hash(buffer: &[u8]) -> usize {
    ((buffer[0] as usize) * 4) + ((buffer[1] as usize) * 5)
}

/// Maximum repetition length supported by PKLib
pub const MAX_REP_LENGTH: usize = 0x204; // 516 bytes

/// Convenience function to compress data in memory
pub fn implode_bytes(
    data: &[u8],
    mode: crate::CompressionMode,
    dict_size: crate::DictionarySize,
) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    {
        let mut writer = ImplodeWriter::new(&mut output, mode, dict_size)?;
        writer.write_all(data)?;
        writer.finish()?;
    }
    Ok(output)
}
