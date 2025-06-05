//! ExplodeReader - Streaming decompression reader
//!
//! This module implements the ExplodeReader that provides a Read interface
//! for PKLib explode decompression, including the main expansion logic.

use super::{state::ExplodeState, *};
use crate::tables::{
    CH_BITS_ASC, CH_CODE_ASC, DIST_BITS, DIST_CODE, EX_LEN_BITS, LEN_BASE, LEN_BITS, LEN_CODE,
};
use crate::{CompressionMode, PkLibError, Result};
use std::io::Read;

/// Streaming decompression reader implementing Read trait
#[derive(Debug)]
pub struct ExplodeReader<R: Read> {
    reader: R,
    state: ExplodeState,
    initialized: bool,
    finished: bool,
    output_buffer: Vec<u8>,
    output_pos: usize,
}

impl<R: Read> ExplodeReader<R> {
    /// Create a new ExplodeReader
    pub fn new(reader: R) -> Result<Self> {
        Ok(Self {
            reader,
            state: ExplodeState::new(),
            initialized: false,
            finished: false,
            output_buffer: Vec::new(),
            output_pos: 0,
        })
    }

    /// Initialize the reader by reading and parsing the header
    fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // Load initial input buffer (similar to PKLib read_buf call)
        self.state.in_bytes = self.reader.read(&mut self.state.in_buff)?;
        if self.state.in_bytes <= 4 {
            return Err(PkLibError::InvalidData("Not enough data".to_string()));
        }

        // Extract header from buffer (like PKLib does)
        self.state.ctype = match self.state.in_buff[0] {
            0 => CompressionMode::Binary,
            1 => CompressionMode::ASCII,
            _ => return Err(PkLibError::InvalidCompressionMode(self.state.in_buff[0])),
        };

        self.state.dsize_bits = self.state.in_buff[1] as u32;
        self.state.bit_buff = self.state.in_buff[2] as u32;
        self.state.extra_bits = 0;
        self.state.in_pos = 3; // Skip header bytes

        // Validate dictionary size
        if self.state.dsize_bits < 4 || self.state.dsize_bits > 6 {
            return Err(PkLibError::InvalidDictionaryBits(
                self.state.dsize_bits as u8,
            ));
        }

        self.state.dsize_mask = 0xFFFF >> (16 - self.state.dsize_bits);

        // Copy static tables
        self.state.dist_bits.copy_from_slice(&DIST_BITS);
        self.state.len_bits.copy_from_slice(&LEN_BITS);
        self.state.ex_len_bits.copy_from_slice(&EX_LEN_BITS);
        self.state.len_base.copy_from_slice(&LEN_BASE);

        // Generate decode tables
        Self::gen_decode_tabs(&mut self.state.length_codes, &LEN_CODE, &LEN_BITS);
        Self::gen_decode_tabs(&mut self.state.dist_pos_codes, &DIST_CODE, &DIST_BITS);

        // Generate ASCII tables if needed
        if matches!(self.state.ctype, CompressionMode::ASCII) {
            self.state.ch_bits_asc.copy_from_slice(&CH_BITS_ASC);
            self.gen_asc_tabs();
        }

        self.initialized = true;
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
            let mut bits_asc = self.state.ch_bits_asc[count];

            if bits_asc <= 8 {
                let add = 1u32 << bits_asc;
                let mut acc = ch_code_asc as u32;

                while acc < 0x100 {
                    if (acc as usize) < self.state.offs_2c34.len() {
                        self.state.offs_2c34[acc as usize] = count as u8;
                    }
                    acc += add;
                }
            } else if (ch_code_asc & 0xFF) != 0 {
                let acc = (ch_code_asc & 0xFF) as usize;
                if acc < self.state.offs_2c34.len() {
                    self.state.offs_2c34[acc] = 0xFF;
                }

                if (ch_code_asc & 0x3F) != 0 {
                    bits_asc -= 4;
                    self.state.ch_bits_asc[count] = bits_asc;

                    let add = 1u32 << bits_asc;
                    let mut acc = (ch_code_asc >> 4) as u32;
                    while acc < 0x100 {
                        if (acc as usize) < self.state.offs_2d34.len() {
                            self.state.offs_2d34[acc as usize] = count as u8;
                        }
                        acc += add;
                    }
                } else {
                    bits_asc -= 6;
                    self.state.ch_bits_asc[count] = bits_asc;

                    let add = 1u32 << bits_asc;
                    let mut acc = (ch_code_asc >> 6) as u32;
                    while acc < 0x80 {
                        if (acc as usize) < self.state.offs_2e34.len() {
                            self.state.offs_2e34[acc as usize] = count as u8;
                        }
                        acc += add;
                    }
                }
            } else {
                bits_asc -= 8;
                self.state.ch_bits_asc[count] = bits_asc;

                let add = 1u32 << bits_asc;
                let mut acc = (ch_code_asc >> 8) as u32;
                while acc < 0x100 {
                    if (acc as usize) < self.state.offs_2eb4.len() {
                        self.state.offs_2eb4[acc as usize] = count as u8;
                    }
                    acc += add;
                }
            }
        }
    }

    /// Main expansion logic - port of Expand function from PKLib
    fn expand(&mut self) -> Result<usize> {
        if !self.initialized {
            self.initialize()?;
        }

        if self.finished {
            return Ok(0);
        }

        let mut bytes_written = 0;
        self.state.output_pos = 0x1000; // Initialize output buffer position

        // Main decompression loop
        loop {
            let next_literal = self.state.decode_lit(&mut self.reader)?;

            match next_literal {
                // End of stream
                LITERAL_END_OF_STREAM => {
                    self.finished = true;
                    break;
                }

                // Error
                LITERAL_ERROR => {
                    return Err(PkLibError::DecompressionError("Decode error".to_string()));
                }

                // Repetition (length encoded as literal >= 0x100)
                literal if literal >= 0x100 => {
                    // Calculate repetition length
                    let rep_length = literal - 0xFE;

                    // Get backward distance to repetition
                    let minus_dist = self.state.decode_dist(&mut self.reader, rep_length)?;
                    if minus_dist == 0 {
                        return Err(PkLibError::DecompressionError(
                            "Invalid distance".to_string(),
                        ));
                    }

                    // Calculate source and target positions
                    let target_pos = self.state.output_pos;
                    let source_pos = target_pos.saturating_sub(minus_dist as usize);

                    // Bounds checking
                    if source_pos >= self.state.out_buff.len()
                        || target_pos + rep_length as usize > self.state.out_buff.len()
                    {
                        return Err(PkLibError::DecompressionError(
                            "Buffer overflow".to_string(),
                        ));
                    }

                    // Copy the repeating sequence (may overlap)
                    for i in 0..rep_length as usize {
                        if target_pos + i < self.state.out_buff.len()
                            && source_pos + i < self.state.out_buff.len()
                        {
                            self.state.out_buff[target_pos + i] =
                                self.state.out_buff[source_pos + i];
                        }
                    }

                    self.state.output_pos += rep_length as usize;
                }

                // Literal byte (< 0x100)
                literal => {
                    if self.state.output_pos < self.state.out_buff.len() {
                        self.state.out_buff[self.state.output_pos] = literal as u8;
                        self.state.output_pos += 1;
                    } else {
                        return Err(PkLibError::DecompressionError(
                            "Output buffer overflow".to_string(),
                        ));
                    }
                }
            }

            // Flush output buffer when it reaches capacity
            if self.state.output_pos >= 0x2000 {
                // Copy decompressed data from second half of buffer to output
                let copy_start = 0x1000;
                let copy_end = self.state.output_pos.min(0x2000);

                if copy_end > copy_start {
                    self.output_buffer
                        .extend_from_slice(&self.state.out_buff[copy_start..copy_end]);
                    bytes_written += copy_end - copy_start;
                }

                // Move remaining data to first half (for repetition references)
                let remaining_bytes = self.state.output_pos - 0x1000;
                if remaining_bytes > 0 && remaining_bytes <= 0x1000 {
                    self.state
                        .out_buff
                        .copy_within(0x1000..self.state.output_pos, 0);
                }
                self.state.output_pos = remaining_bytes;

                // Return what we have so far
                if bytes_written > 0 {
                    break;
                }
            }
        }

        // Flush any remaining data
        if self.finished && self.state.output_pos > 0x1000 {
            let copy_start = 0x1000;
            let copy_end = self.state.output_pos;

            if copy_end > copy_start {
                self.output_buffer
                    .extend_from_slice(&self.state.out_buff[copy_start..copy_end]);
                bytes_written += copy_end - copy_start;
            }
        }

        Ok(bytes_written)
    }
}

impl<R: Read> Read for ExplodeReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // If we have buffered output, return that first
        if self.output_pos < self.output_buffer.len() {
            let available = self.output_buffer.len() - self.output_pos;
            let to_copy = buf.len().min(available);
            buf[..to_copy]
                .copy_from_slice(&self.output_buffer[self.output_pos..self.output_pos + to_copy]);
            self.output_pos += to_copy;

            // Clear consumed data periodically to avoid unbounded growth
            if self.output_pos >= self.output_buffer.len() {
                self.output_buffer.clear();
                self.output_pos = 0;
            }

            return Ok(to_copy);
        }

        if self.finished {
            return Ok(0);
        }

        // Expand more data
        match self.expand() {
            Ok(0) => Ok(0), // EOF
            Ok(_) => {
                // Try to return data from newly expanded buffer
                if self.output_pos < self.output_buffer.len() {
                    let available = self.output_buffer.len() - self.output_pos;
                    let to_copy = buf.len().min(available);
                    buf[..to_copy].copy_from_slice(
                        &self.output_buffer[self.output_pos..self.output_pos + to_copy],
                    );
                    self.output_pos += to_copy;
                    Ok(to_copy)
                } else {
                    Ok(0)
                }
            }
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        }
    }
}
