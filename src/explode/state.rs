//! Decompression state management
//!
//! This module manages the internal state for PKLib explode decompression,
//! matching the TDcmpStruct from the original PKLib implementation.

use super::{
    CH_BITS_ASC_SIZE, CODES_SIZE, DIST_SIZES, IN_BUFF_SIZE, LENS_SIZES, OFFSS_SIZE, OFFSS_SIZE1,
    OUT_BUFF_SIZE,
};
use crate::tables::{
    CH_BITS_ASC, CH_CODE_ASC, DIST_BITS, DIST_CODE, EX_LEN_BITS, LEN_BASE, LEN_BITS, LEN_CODE,
};
use crate::{CompressionMode, PkLibError, Result};

/// Decompression state structure matching PKLib's TDcmpStruct
#[derive(Debug)]
pub struct ExplodeState {
    // Core state
    /// Compression mode (Binary or ASCII)
    pub ctype: CompressionMode,
    /// Current position in output buffer
    pub output_pos: usize,
    /// Number of bits for dictionary size (4, 5, or 6)
    pub dsize_bits: u32,
    /// Bit mask for dictionary size
    pub dsize_mask: u32,
    /// Bit buffer for processing input data
    pub bit_buff: u32,
    /// Number of extra bits in bit buffer
    pub extra_bits: u32,
    /// Current position in input buffer
    pub in_pos: usize,
    /// Number of bytes available in input buffer
    pub in_bytes: usize,

    // Buffers
    /// Output circular buffer
    pub out_buff: [u8; OUT_BUFF_SIZE],
    /// Input buffer for reading compressed data
    pub in_buff: [u8; IN_BUFF_SIZE],

    // Decode tables (generated at runtime)
    /// Distance position codes for decoding
    pub dist_pos_codes: [u8; CODES_SIZE],
    /// Length codes for decoding
    pub length_codes: [u8; CODES_SIZE],
    /// ASCII decode table 1
    pub offs_2c34: [u8; OFFSS_SIZE],
    /// ASCII decode table 2
    pub offs_2d34: [u8; OFFSS_SIZE],
    /// ASCII decode table 3
    pub offs_2e34: [u8; OFFSS_SIZE1],
    /// ASCII decode table 4
    pub offs_2eb4: [u8; OFFSS_SIZE],
    /// ASCII character bit lengths
    pub ch_bits_asc: [u8; CH_BITS_ASC_SIZE],

    // Static tables (copied from global tables)
    /// Distance bit lengths
    pub dist_bits: [u8; DIST_SIZES],
    /// Length bit lengths
    pub len_bits: [u8; LENS_SIZES],
    /// Extra length bits
    pub ex_len_bits: [u8; LENS_SIZES],
    /// Length base values
    pub len_base: [u16; LENS_SIZES],
}

impl ExplodeState {
    /// Create a new decompression state
    pub fn new() -> Self {
        Self {
            ctype: CompressionMode::Binary,
            output_pos: 0x1000, // Initialize to PKLib default
            dsize_bits: 0,
            dsize_mask: 0,
            bit_buff: 0,
            extra_bits: 0,
            in_pos: 0,
            in_bytes: 0,
            out_buff: [0; OUT_BUFF_SIZE],
            in_buff: [0; IN_BUFF_SIZE],
            dist_pos_codes: [0; CODES_SIZE],
            length_codes: [0; CODES_SIZE],
            offs_2c34: [0; OFFSS_SIZE],
            offs_2d34: [0; OFFSS_SIZE],
            offs_2e34: [0; OFFSS_SIZE1],
            offs_2eb4: [0; OFFSS_SIZE],
            ch_bits_asc: [0; CH_BITS_ASC_SIZE],
            dist_bits: [0; DIST_SIZES],
            len_bits: [0; LENS_SIZES],
            ex_len_bits: [0; LENS_SIZES],
            len_base: [0; LENS_SIZES],
        }
    }

    /// Initialize state from compressed data header
    pub fn initialize(&mut self, header_data: &[u8]) -> Result<()> {
        if header_data.len() < 4 {
            return Err(PkLibError::InvalidData("Header too short".to_string()));
        }

        // Read header
        self.ctype = match header_data[0] {
            0 => CompressionMode::Binary,
            1 => CompressionMode::ASCII,
            _ => return Err(PkLibError::InvalidCompressionMode(header_data[0])),
        };

        self.dsize_bits = header_data[1] as u32;
        self.bit_buff = header_data[2] as u32;
        self.extra_bits = 0;
        self.in_pos = 3;

        // Validate dictionary size
        if self.dsize_bits < 4 || self.dsize_bits > 6 {
            return Err(PkLibError::InvalidDictionaryBits(self.dsize_bits as u8));
        }

        self.dsize_mask = 0xFFFF >> (16 - self.dsize_bits);

        // Copy static tables
        self.dist_bits.copy_from_slice(&DIST_BITS);
        self.len_bits.copy_from_slice(&LEN_BITS);
        self.ex_len_bits.copy_from_slice(&EX_LEN_BITS);
        self.len_base.copy_from_slice(&LEN_BASE);

        // Generate decode tables
        Self::gen_decode_tabs(&mut self.length_codes, &LEN_CODE, &LEN_BITS);
        Self::gen_decode_tabs(&mut self.dist_pos_codes, &DIST_CODE, &DIST_BITS);

        // Generate ASCII tables if needed
        if matches!(self.ctype, CompressionMode::ASCII) {
            self.ch_bits_asc.copy_from_slice(&CH_BITS_ASC);
            self.gen_asc_tabs();
        }

        Ok(())
    }

    /// Generate decode tables (port of GenDecodeTabs from PKLib)
    fn gen_decode_tabs(positions: &mut [u8], start_indexes: &[u8], length_bits: &[u8]) {
        for i in 0..start_indexes.len() {
            let length = 1u32 << length_bits[i];
            let mut index = start_indexes[i] as u32;

            while index < 0x100 {
                if (index as usize) < positions.len() {
                    positions[index as usize] = i as u8;
                }
                index += length;
            }
        }
    }

    /// Generate ASCII decode tables (port of GenAscTabs from PKLib)
    fn gen_asc_tabs(&mut self) {
        for count in (0..=0xFF).rev() {
            let ch_code_asc = CH_CODE_ASC[count];
            let mut bits_asc = self.ch_bits_asc[count];

            if bits_asc <= 8 {
                let add = 1u32 << bits_asc;
                let mut acc = ch_code_asc as u32;

                while acc < 0x100 {
                    if (acc as usize) < self.offs_2c34.len() {
                        self.offs_2c34[acc as usize] = count as u8;
                    }
                    acc += add;
                }
            } else if (ch_code_asc & 0xFF) != 0 {
                let acc = (ch_code_asc & 0xFF) as usize;
                if acc < self.offs_2c34.len() {
                    self.offs_2c34[acc] = 0xFF;
                }

                if (ch_code_asc & 0x3F) != 0 {
                    bits_asc -= 4;
                    self.ch_bits_asc[count] = bits_asc;

                    let add = 1u32 << bits_asc;
                    let mut acc = (ch_code_asc >> 4) as u32;
                    while acc < 0x100 {
                        if (acc as usize) < self.offs_2d34.len() {
                            self.offs_2d34[acc as usize] = count as u8;
                        }
                        acc += add;
                    }
                } else {
                    bits_asc -= 6;
                    self.ch_bits_asc[count] = bits_asc;

                    let add = 1u32 << bits_asc;
                    let mut acc = (ch_code_asc >> 6) as u32;
                    while acc < 0x80 {
                        if (acc as usize) < self.offs_2e34.len() {
                            self.offs_2e34[acc as usize] = count as u8;
                        }
                        acc += add;
                    }
                }
            } else {
                bits_asc -= 8;
                self.ch_bits_asc[count] = bits_asc;

                let add = 1u32 << bits_asc;
                let mut acc = (ch_code_asc >> 8) as u32;
                while acc < 0x100 {
                    if (acc as usize) < self.offs_2eb4.len() {
                        self.offs_2eb4[acc as usize] = count as u8;
                    }
                    acc += add;
                }
            }
        }
    }
}

impl Default for ExplodeState {
    fn default() -> Self {
        Self::new()
    }
}
