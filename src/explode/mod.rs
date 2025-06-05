//! PKLib Explode (decompression) implementation
//!
//! This module provides decompression functionality compatible with the PKWare DCL format.
//! It implements the explode algorithm exactly as specified in the original PKLib.

mod decoder;
mod reader;
mod state;

pub use reader::ExplodeReader;
pub use state::ExplodeState;

use crate::Result;
use std::io::Read;

/// Input buffer size for decompression (2048 bytes)
pub const IN_BUFF_SIZE: usize = 0x800;

/// Output buffer size for decompression (8708 bytes)
pub const OUT_BUFF_SIZE: usize = 0x2204;

/// Size of literal code arrays (256 bytes)
pub const CODES_SIZE: usize = 0x100;

/// Size of offset arrays (256 bytes)
pub const OFFSS_SIZE: usize = 0x100;

/// Size of smaller offset arrays (128 bytes)
pub const OFFSS_SIZE1: usize = 0x80;

/// Size of ASCII character bits array (256 bytes)
pub const CH_BITS_ASC_SIZE: usize = 0x100;

/// Number of distance codes (64)
pub const DIST_SIZES: usize = 0x40;

/// Number of length codes (16)
pub const LENS_SIZES: usize = 0x10;

/// Decompression completed successfully
pub const PKDCL_OK: u32 = 0;

/// End of compressed stream reached
pub const PKDCL_STREAM_END: u32 = 1;

/// Dictionary required for decompression
pub const PKDCL_NEED_DICT: u32 = 2;

/// Continue decompression operation
pub const PKDCL_CONTINUE: u32 = 10;

/// More input data required
pub const PKDCL_GET_INPUT: u32 = 11;

/// End of stream literal marker (0x305)
pub const LITERAL_END_OF_STREAM: u32 = 0x305;

/// Literal decoding error marker (0x306)
pub const LITERAL_ERROR: u32 = 0x306;

/// Convenience function to decompress data in memory
pub fn explode_bytes(data: &[u8]) -> Result<Vec<u8>> {
    let mut reader = ExplodeReader::new(std::io::Cursor::new(data))?;
    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    Ok(output)
}
