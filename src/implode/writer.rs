//! ImplodeWriter - Streaming compression writer
//!
//! This module implements the ImplodeWriter that provides a Write interface
//! for PKLib implode compression, including bit encoding and output management.

use super::{pattern::MatchResult, state::ImplodeState};
use crate::{CompressionMode, DictionarySize, PkLibError, Result};
use std::io::Write;

/// Streaming compression writer implementing Write trait
#[derive(Debug)]
pub struct ImplodeWriter<W: Write> {
    writer: W,
    state: ImplodeState,
    initialized: bool,
    finished: bool,
    input_buffer: Vec<u8>,
}

impl<W: Write> ImplodeWriter<W> {
    /// Create a new ImplodeWriter
    pub fn new(writer: W, mode: CompressionMode, dict_size: DictionarySize) -> Result<Self> {
        let state = ImplodeState::new(mode, dict_size)?;
        Ok(Self {
            writer,
            state,
            initialized: false,
            finished: false,
            input_buffer: Vec::new(),
        })
    }

    /// Initialize the writer by setting up the output buffer like PKLib
    fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // PKLib initializes the output buffer with compression type and dictionary size
        // Store the compression type and dictionary size (PKLib lines 419-421)
        self.state.out_buff[0] = self.state.ctype as u8;
        self.state.out_buff[1] = self.state.dsize_bits as u8;
        self.state.out_bytes = 2;

        // Reset output buffer from position 2 onwards (PKLib lines 424-425)
        for i in 2..self.state.out_buff.len() {
            self.state.out_buff[i] = 0;
        }
        self.state.out_bits = 0;

        self.initialized = true;
        Ok(())
    }

    /// Finish compression and flush all remaining data
    pub fn finish(mut self) -> Result<W> {
        if !self.finished {
            // Ensure initialization even for empty data (like PKLib)
            if !self.initialized {
                self.initialize()?;
            }
            self.flush_remaining_data()?;
            self.write_end_marker()?;
            self.flush_output_buffer()?;
            self.finished = true;
        }

        // Use ManuallyDrop to avoid Drop being called when we move the writer out
        use std::mem::ManuallyDrop;
        let writer = unsafe {
            let manual_drop_self = ManuallyDrop::new(self);
            std::ptr::read(&manual_drop_self.writer)
        };
        Ok(writer)
    }

    /// Process accumulated input data
    fn process_input(&mut self) -> Result<()> {
        if !self.initialized {
            self.initialize()?;
        }

        // Move input data to work buffer
        let input_len = self.input_buffer.len();
        if input_len == 0 {
            return Ok(());
        }

        // Ensure we have space in work buffer
        let available_space = self.state.work_buff.len() - self.state.work_bytes;
        let copy_len = input_len.min(available_space);

        if copy_len > 0 {
            self.state.work_buff[self.state.work_bytes..self.state.work_bytes + copy_len]
                .copy_from_slice(&self.input_buffer[..copy_len]);
            self.state.work_bytes += copy_len;

            // Remove processed data from input buffer
            self.input_buffer.drain(..copy_len);
        }

        // Build hash table for the current buffer
        if self.state.work_bytes > 1 {
            self.state.sort_buffer(0, self.state.work_bytes);

            // Compress the data
            self.compress_buffer()?;
        }

        Ok(())
    }

    /// Compress data in the work buffer
    fn compress_buffer(&mut self) -> Result<()> {
        let mut pos = 0;

        while pos < self.state.work_bytes.saturating_sub(1) {
            // Try to find a repetition at current position
            let match_result = self.state.find_repetition(pos);

            if match_result.is_match() {
                // Encode the match
                self.encode_match(match_result)?;
                pos += match_result.length;
            } else {
                // Encode literal byte
                self.encode_literal(self.state.work_buff[pos])?;
                pos += 1;
            }
        }

        // Handle last byte if any
        if pos < self.state.work_bytes {
            self.encode_literal(self.state.work_buff[pos])?;
        }

        Ok(())
    }

    /// Encode a literal byte
    fn encode_literal(&mut self, byte: u8) -> Result<()> {
        let literal_index = byte as usize;
        if literal_index < self.state.literal_bits.len() {
            let bits = self.state.literal_bits[literal_index];
            let code = self.state.literal_codes[literal_index] as u32;
            self.output_bits(bits as u32, code)?;
        } else {
            return Err(PkLibError::InvalidData("Invalid literal value".to_string()));
        }
        Ok(())
    }

    /// Encode a length/distance match
    fn encode_match(&mut self, match_result: MatchResult) -> Result<()> {
        let length = match_result.length;
        let distance = match_result.distance;

        // Encode length (PKLib uses length + 0xFE for the encoding)
        let length_code = length + 0xFE;
        if length_code < self.state.literal_bits.len() {
            let bits = self.state.literal_bits[length_code];
            let code = self.state.literal_codes[length_code] as u32;
            self.output_bits(bits as u32, code)?;
        } else {
            return Err(PkLibError::InvalidLength(length as u32));
        }

        // Encode distance
        let dist_minus_one = (distance - 1) as u32; // PKLib stores distance - 1
        if length == 2 {
            // For 2-byte repetitions, use special encoding
            let dist_code_index = (dist_minus_one >> 2) as usize;
            if dist_code_index < self.state.dist_bits.len() {
                let bits = self.state.dist_bits[dist_code_index];
                let code = self.state.dist_codes[dist_code_index] as u32;
                self.output_bits(bits as u32, code)?;
                self.output_bits(2, dist_minus_one & 3)?;
            } else {
                return Err(PkLibError::InvalidDistance(distance as u32));
            }
        } else {
            // For longer repetitions, use dictionary size bits
            let dist_code_index = (dist_minus_one >> self.state.dsize_bits) as usize;
            if dist_code_index < self.state.dist_bits.len() {
                let bits = self.state.dist_bits[dist_code_index];
                let code = self.state.dist_codes[dist_code_index] as u32;
                self.output_bits(bits as u32, code)?;
                self.output_bits(
                    self.state.dsize_bits,
                    dist_minus_one & self.state.dsize_mask,
                )?;
            } else {
                return Err(PkLibError::InvalidDistance(distance as u32));
            }
        }

        Ok(())
    }

    /// Output bits to the compressed stream (exact port of OutputBits from PKLib)
    fn output_bits(&mut self, mut n_bits: u32, mut bit_buffer: u32) -> Result<()> {
        // If more than 8 bits to output, do recursion (exactly like PKLib)
        if n_bits > 8 {
            self.output_bits(8, bit_buffer)?;
            bit_buffer >>= 8;
            n_bits -= 8;
            return self.output_bits(n_bits, bit_buffer);
        }

        // Add bits to the last out byte in out_buff (PKLib line 124)
        let out_bits = self.state.out_bits;
        let out_bytes = self.state.out_bytes as usize;

        // Ensure we have space in the buffer
        if out_bytes >= self.state.out_buff.len() {
            self.flush_output_buffer()?;
            return self.output_bits(n_bits, bit_buffer);
        }

        // PKLib: pWork->out_buff[pWork->out_bytes] |= (unsigned char)(bit_buff << out_bits);
        self.state.out_buff[out_bytes] |= ((bit_buffer << out_bits) & 0xFF) as u8;
        self.state.out_bits += n_bits;

        // If 8 or more bits, increment number of bytes (PKLib lines 128-141)
        if self.state.out_bits > 8 {
            self.state.out_bytes += 1;

            // PKLib: bit_buff >>= (8 - out_bits);
            bit_buffer >>= 8 - out_bits;

            // Ensure we have space for the next byte
            let new_out_bytes = self.state.out_bytes as usize;
            if new_out_bytes < self.state.out_buff.len() {
                // PKLib: pWork->out_buff[pWork->out_bytes] = (unsigned char)bit_buff;
                self.state.out_buff[new_out_bytes] = (bit_buffer & 0xFF) as u8;
            }

            // PKLib: pWork->out_bits &= 7;
            self.state.out_bits &= 7;
        } else {
            // PKLib: pWork->out_bits &= 7;
            self.state.out_bits &= 7;
            if self.state.out_bits == 0 {
                self.state.out_bytes += 1;
            }
        }

        // If there is enough compressed bytes, flush them (PKLib lines 144-145)
        if self.state.out_bytes >= 0x800 {
            self.flush_output_buffer()?;
        }

        Ok(())
    }

    /// Flush the output buffer to the writer
    fn flush_output_buffer(&mut self) -> Result<()> {
        if self.state.out_bytes > 0 {
            let bytes_to_write = self.state.out_bytes as usize;
            if bytes_to_write <= self.state.out_buff.len() {
                self.writer
                    .write_all(&self.state.out_buff[..bytes_to_write])?;

                // Clear the buffer but preserve any partial byte
                let save_byte =
                    if self.state.out_bits > 0 && bytes_to_write < self.state.out_buff.len() {
                        self.state.out_buff[bytes_to_write]
                    } else {
                        0
                    };

                self.state.out_buff.fill(0);
                self.state.out_bytes = 0;

                if self.state.out_bits > 0 {
                    self.state.out_buff[0] = save_byte;
                }
            }
        }
        Ok(())
    }

    /// Write end-of-stream marker
    fn write_end_marker(&mut self) -> Result<()> {
        // PKLib uses literal code 0x305 as end marker
        const END_MARKER: usize = 0x305;
        if END_MARKER < self.state.literal_bits.len() {
            let bits = self.state.literal_bits[END_MARKER];
            let code = self.state.literal_codes[END_MARKER] as u32;
            self.output_bits(bits as u32, code)?;
        }

        // Flush any remaining bits
        if self.state.out_bits > 0 {
            self.state.out_bytes += 1;
        }

        Ok(())
    }

    /// Flush any remaining input data
    fn flush_remaining_data(&mut self) -> Result<()> {
        if !self.input_buffer.is_empty() {
            self.process_input()?;
        }
        Ok(())
    }
}

impl<W: Write> Write for ImplodeWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Accumulate input data
        self.input_buffer.extend_from_slice(buf);

        // Process when we have enough data
        if self.input_buffer.len() >= 4096 {
            // Process in chunks
            self.process_input()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.process_input()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.flush_output_buffer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.writer.flush()
    }
}

impl<W: Write> Drop for ImplodeWriter<W> {
    fn drop(&mut self) {
        if !self.finished {
            // Try to finish compression, but ignore errors in drop
            let _ = self.flush_remaining_data();
            let _ = self.write_end_marker();
            let _ = self.flush_output_buffer();
        }
    }
}
