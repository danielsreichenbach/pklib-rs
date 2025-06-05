//! Bit decoding and literal/distance decoding functions
//!
//! This module implements the core decoding logic for PKLib explode,
//! including bit manipulation and Huffman-style decoding.

use super::{state::ExplodeState, *};
use crate::{CompressionMode, Result};
use std::io::Read;

impl ExplodeState {
    /// Remove given number of bits from bit buffer, loading new data if needed
    /// Port of WasteBits function from PKLib explode.c
    pub fn waste_bits<R: Read>(&mut self, reader: &mut R, n_bits: u32) -> Result<u32> {
        // If we have enough bits in buffer
        if n_bits <= self.extra_bits {
            self.extra_bits -= n_bits;
            self.bit_buff >>= n_bits;
            return Ok(PKDCL_OK);
        }

        // Shift out the remaining bits
        self.bit_buff >>= self.extra_bits;

        // Load input buffer if necessary
        if self.in_pos >= self.in_bytes {
            self.in_pos = 0;
            self.in_bytes = reader.read(&mut self.in_buff)?;
            if self.in_bytes == 0 {
                return Ok(PKDCL_STREAM_END);
            }
        }

        // Update bit buffer with new byte
        if self.in_pos < self.in_bytes {
            self.bit_buff |= (self.in_buff[self.in_pos] as u32) << 8;
            self.in_pos += 1;
        }

        // Shift right by remaining bits needed
        self.bit_buff >>= n_bits - self.extra_bits;
        self.extra_bits = (self.extra_bits + 8) - n_bits;

        Ok(PKDCL_OK)
    }

    /// Decode next literal from compressed data
    /// Port of DecodeLit function from PKLib explode.c
    ///
    /// Returns:
    /// - 0x000-0x0FF: Literal byte values
    /// - 0x100-0x304: Repetition length (length = value - 0xFE)
    /// - 0x305: End of stream
    /// - 0x306: Error
    pub fn decode_lit<R: Read>(&mut self, reader: &mut R) -> Result<u32> {
        // Test the current bit in buffer
        if (self.bit_buff & 1) != 0 {
            // Remove one bit from input data
            if self.waste_bits(reader, 1)? != PKDCL_OK {
                return Ok(LITERAL_ERROR);
            }

            // Next 8 bits hold index to length code table
            let length_code = self.length_codes[(self.bit_buff & 0xFF) as usize] as usize;

            // Remove the appropriate number of bits
            if self.waste_bits(reader, self.len_bits[length_code] as u32)? != PKDCL_OK {
                return Ok(LITERAL_ERROR);
            }

            // Check for extra bits for this length code
            let extra_length_bits = self.ex_len_bits[length_code];
            let mut final_length_code = length_code;

            if extra_length_bits != 0 {
                let extra_length = self.bit_buff & ((1 << extra_length_bits) - 1);

                if self.waste_bits(reader, extra_length_bits as u32)? != PKDCL_OK
                    && (length_code + extra_length as usize) != 0x10E
                {
                    return Ok(LITERAL_ERROR);
                }
                final_length_code = (self.len_base[length_code] as usize) + (extra_length as usize);
            }

            // Add 0x100 to distinguish from uncompressed bytes
            return Ok(final_length_code as u32 + 0x100);
        }

        // Remove one bit from input data
        if self.waste_bits(reader, 1)? != PKDCL_OK {
            return Ok(LITERAL_ERROR);
        }

        // Binary compression: read 8 bits directly
        if matches!(self.ctype, CompressionMode::Binary) {
            let uncompressed_byte = self.bit_buff & 0xFF;
            if self.waste_bits(reader, 8)? != PKDCL_OK {
                return Ok(LITERAL_ERROR);
            }
            return Ok(uncompressed_byte);
        }

        // ASCII compression: use decode tables
        let value = if (self.bit_buff & 0xFF) != 0 {
            let mut val = self.offs_2c34[(self.bit_buff & 0xFF) as usize] as u32;

            if val == 0xFF {
                if (self.bit_buff & 0x3F) != 0 {
                    if self.waste_bits(reader, 4)? != PKDCL_OK {
                        return Ok(LITERAL_ERROR);
                    }
                    val = self.offs_2d34[(self.bit_buff & 0xFF) as usize] as u32;
                } else {
                    if self.waste_bits(reader, 6)? != PKDCL_OK {
                        return Ok(LITERAL_ERROR);
                    }
                    val = self.offs_2e34[(self.bit_buff & 0x7F) as usize] as u32;
                }
            }
            val
        } else {
            if self.waste_bits(reader, 8)? != PKDCL_OK {
                return Ok(LITERAL_ERROR);
            }
            self.offs_2eb4[(self.bit_buff & 0xFF) as usize] as u32
        };

        // Final bit consumption for ASCII character
        if self.waste_bits(reader, self.ch_bits_asc[value as usize] as u32)? != PKDCL_OK {
            Ok(LITERAL_ERROR)
        } else {
            Ok(value)
        }
    }

    /// Decode distance for repetition
    /// Port of DecodeDist function from PKLib explode.c
    pub fn decode_dist<R: Read>(&mut self, reader: &mut R, rep_length: u32) -> Result<u32> {
        // Get distance position code from next 2-8 bits
        let dist_pos_code = self.dist_pos_codes[(self.bit_buff & 0xFF) as usize];
        let dist_pos_bits = self.dist_bits[dist_pos_code as usize];

        if self.waste_bits(reader, dist_pos_bits as u32)? != PKDCL_OK {
            return Ok(0);
        }

        let distance = if rep_length == 2 {
            // For 2-byte repetitions, take 2 bits for distance
            let dist = ((dist_pos_code as u32) << 2) | (self.bit_buff & 0x03);
            if self.waste_bits(reader, 2)? != PKDCL_OK {
                return Ok(0);
            }
            dist
        } else {
            // For longer repetitions, take dsize_bits bits for distance
            let dist =
                ((dist_pos_code as u32) << self.dsize_bits) | (self.bit_buff & self.dsize_mask);
            if self.waste_bits(reader, self.dsize_bits)? != PKDCL_OK {
                return Ok(0);
            }
            dist
        };

        Ok(distance + 1)
    }
}
