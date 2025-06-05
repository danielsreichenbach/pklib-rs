//! Compression state management
//!
//! This module manages the internal state for PKLib implode compression,
//! matching the TCmpStruct from the original PKLib implementation.

use super::{HASH_TABLE_SIZE, LITERALS_COUNT, OFFSS_SIZE2, OUT_BUFF_SIZE, WORK_BUFF_SIZE};
use crate::tables::{
    CH_BITS_ASC, CH_CODE_ASC, DIST_BITS, DIST_CODE, EX_LEN_BITS, LEN_BITS, LEN_CODE,
};
use crate::{CompressionMode, DictionarySize, Result};

/// Compression state structure matching PKLib's TCmpStruct
#[derive(Debug)]
pub struct ImplodeState {
    /// Current backward distance of found repetition (decreased by 1)
    pub distance: u32,
    /// Number of bytes available in output buffer
    pub out_bytes: u32,
    /// Number of bits available in the last output byte
    pub out_bits: u32,
    /// Number of bits needed for dictionary size (4/5/6)
    pub dsize_bits: u32,
    /// Bit mask for dictionary (0x0F/0x1F/0x3F)
    pub dsize_mask: u32,
    /// Compression type (Binary/ASCII)
    pub ctype: CompressionMode,
    /// Dictionary size in bytes
    pub dsize_bytes: u32,

    // Static tables (copied from global tables)
    /// Distance bit lengths
    pub dist_bits: [u8; 64],
    /// Distance codes
    pub dist_codes: [u8; 64],
    /// Table of literal bit lengths for output stream
    pub literal_bits: [u8; LITERALS_COUNT],
    /// Table of literal codes for output stream
    pub literal_codes: [u16; LITERALS_COUNT],

    // Working buffers
    /// Hash table: indexes to phash_offs for each PAIR_HASH
    pub phash_to_index: [u16; HASH_TABLE_SIZE],
    /// Output buffer for compressed data
    pub out_buff: [u8; OUT_BUFF_SIZE],
    /// Work buffer (dictionary + uncompressed data)
    pub work_buff: [u8; WORK_BUFF_SIZE],
    /// Table of offsets for each PAIR_HASH
    pub phash_offs: [u16; WORK_BUFF_SIZE],
    /// Temporary offset buffer for optimization
    pub offs_buffer: [u16; OFFSS_SIZE2],

    // Internal state
    /// Current position in work buffer
    pub work_pos: usize,
    /// Current position in input data
    pub input_pos: usize,
    /// Amount of data in work buffer
    pub work_bytes: usize,
}

impl ImplodeState {
    /// Create a new compression state
    pub fn new(mode: CompressionMode, dict_size: DictionarySize) -> Result<Self> {
        let dsize_bytes = dict_size as u32;
        let dsize_bits = dict_size.bits() as u32;
        let dsize_mask = (1u32 << dsize_bits) - 1;

        let mut state = Self {
            distance: 0,
            out_bytes: 0,
            out_bits: 0,
            dsize_bits,
            dsize_mask,
            ctype: mode,
            dsize_bytes,
            dist_bits: [0; 64],
            dist_codes: [0; 64],
            literal_bits: [0; LITERALS_COUNT],
            literal_codes: [0; LITERALS_COUNT],
            phash_to_index: [0; HASH_TABLE_SIZE],
            out_buff: [0; OUT_BUFF_SIZE],
            work_buff: [0; WORK_BUFF_SIZE],
            phash_offs: [0; WORK_BUFF_SIZE],
            offs_buffer: [0; OFFSS_SIZE2],
            work_pos: 0,
            input_pos: 0,
            work_bytes: 0,
        };

        // Copy static tables
        state.dist_bits.copy_from_slice(&DIST_BITS);
        state.dist_codes.copy_from_slice(&DIST_CODE);

        // Initialize literal tables (will be filled during compression)
        state.init_literal_tables()?;

        Ok(state)
    }

    /// Initialize literal encoding tables based on compression mode
    /// This matches PKLib's exact algorithm from implode.c lines 638-667
    fn init_literal_tables(&mut self) -> Result<()> {
        let mut n_count = match self.ctype {
            CompressionMode::Binary => {
                // Binary mode: 9 bits per literal, codes are n*2
                for i in 0..0x100 {
                    self.literal_bits[i] = 9;
                    self.literal_codes[i] = (i * 2) as u16;
                }
                0x100
            }
            CompressionMode::ASCII => {
                // ASCII mode: use PKLib's ChBitsAsc and ChCodeAsc tables
                for i in 0..0x100 {
                    self.literal_bits[i] = CH_BITS_ASC[i] + 1;
                    self.literal_codes[i] = CH_CODE_ASC[i] * 2;
                }
                0x100
            }
        };

        // Build length codes using PKLib's exact algorithm (lines 659-667)
        for i in 0..0x10 {
            let n_count2_max = 1 << EX_LEN_BITS[i];
            for n_count2 in 0..n_count2_max {
                if n_count >= LITERALS_COUNT {
                    break;
                }

                self.literal_bits[n_count] = EX_LEN_BITS[i] + LEN_BITS[i] + 1;
                self.literal_codes[n_count] =
                    (n_count2 << (LEN_BITS[i] + 1)) | ((LEN_CODE[i] as u16 & 0x00FF) * 2) | 1;
                n_count += 1;
            }
        }

        Ok(())
    }

    /// Reset state for new compression
    pub fn reset(&mut self) {
        self.distance = 0;
        self.out_bytes = 0;
        self.out_bits = 0;
        self.work_pos = 0;
        self.input_pos = 0;
        self.work_bytes = 0;

        // Clear buffers
        self.phash_to_index.fill(0);
        self.out_buff.fill(0);
        self.work_buff.fill(0);
        self.phash_offs.fill(0);
        self.offs_buffer.fill(0);
    }

    /// Get current compression statistics
    pub fn stats(&self) -> CompressionStats {
        CompressionStats {
            bytes_processed: self.input_pos,
            compressed_bytes: self.out_bytes as usize,
            compression_ratio: if self.input_pos > 0 {
                (self.out_bytes as f64) / (self.input_pos as f64)
            } else {
                0.0
            },
        }
    }
}

/// Compression statistics
#[derive(Debug, Clone)]
pub struct CompressionStats {
    /// Total bytes processed from input
    pub bytes_processed: usize,
    /// Total compressed bytes produced
    pub compressed_bytes: usize,
    /// Compression ratio (compressed/original)
    pub compression_ratio: f64,
}

impl Default for ImplodeState {
    fn default() -> Self {
        Self::new(CompressionMode::Binary, DictionarySize::Size2K)
            .expect("Failed to create default ImplodeState")
    }
}
