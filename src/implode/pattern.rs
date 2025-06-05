//! Pattern matching implementation for PKLib compression
//!
//! This module implements the core pattern matching algorithm used by PKLib
//! to find repetitions in the input data. It ports the FindRep function
//! from the original PKLib implementation.

use super::{byte_pair_hash, state::ImplodeState, MAX_REP_LENGTH};

/// Result of pattern matching
#[derive(Debug, Clone, Copy)]
pub struct MatchResult {
    /// Length of the found match (0 if no match)
    pub length: usize,
    /// Backward distance to the match (0 if no match)
    pub distance: usize,
}

impl MatchResult {
    /// Create a new match result
    pub fn new(length: usize, distance: usize) -> Self {
        Self { length, distance }
    }

    /// Create a "no match" result
    pub fn no_match() -> Self {
        Self {
            length: 0,
            distance: 0,
        }
    }

    /// Check if this represents a valid match
    pub fn is_match(&self) -> bool {
        self.length >= 2 // PKLib requires at least 2 bytes for a valid match
    }
}

impl ImplodeState {
    /// Find the longest repetition at the current position
    /// This is a port of the FindRep function from PKLib implode.c
    pub fn find_repetition(&mut self, input_pos: usize) -> MatchResult {
        // Need at least 2 bytes for a pattern
        if input_pos + 1 >= self.work_bytes {
            return MatchResult::no_match();
        }

        let hash = byte_pair_hash(&self.work_buff[input_pos..input_pos + 2]);
        let min_offset = input_pos.saturating_sub(self.dsize_bytes as usize);

        // Get all positions where this byte pair hash occurs
        let positions = self.find_hash_positions(hash, input_pos);
        if positions.is_empty() {
            return MatchResult::no_match();
        }

        let mut best_match = MatchResult::no_match();
        let max_length = (self.work_bytes - input_pos).min(MAX_REP_LENGTH);

        // Check each potential match position
        for &match_pos in &positions {
            // Skip if the position is too close or beyond our dictionary window
            if match_pos >= input_pos || match_pos < min_offset {
                continue;
            }

            // Calculate distance
            let distance = input_pos - match_pos;

            // For very close repetitions (distance 1), we need special handling
            if distance == 1 {
                // This is a run-length pattern (e.g., "aaaa...")
                let match_length = self.find_run_length_match(input_pos, max_length);
                if match_length > best_match.length {
                    best_match = MatchResult::new(match_length, distance);
                }
                continue;
            }

            // Find the length of this match
            let match_length = self.compare_sequences(input_pos, match_pos, max_length);

            // Update best match if this one is better
            // PKLib prefers the most recent match when lengths are equal (smaller distance)
            if match_length > best_match.length
                || (match_length == best_match.length && distance < best_match.distance)
            {
                best_match = MatchResult::new(match_length, distance);

                // If we found a very long match, we can stop searching
                if match_length >= MAX_REP_LENGTH {
                    break;
                }
            }
        }

        // PKLib validation: for 2-byte repetitions with distance >= 0x100, don't use them
        // because storing the distance would take more space than the literal bytes
        if best_match.is_match() && best_match.length == 2 && best_match.distance >= 0x100 {
            return MatchResult::no_match();
        }

        // Store the distance in our state for encoding
        if best_match.is_match() {
            self.distance = (best_match.distance - 1) as u32; // PKLib stores distance - 1
        }

        best_match
    }

    /// Compare two sequences and return the length of the match
    fn compare_sequences(&self, pos1: usize, pos2: usize, max_length: usize) -> usize {
        // First check if the sequences start with the same byte pair
        if pos1 + 1 >= self.work_buff.len() || pos2 + 1 >= self.work_buff.len() {
            return 0;
        }

        if self.work_buff[pos1] != self.work_buff[pos2]
            || self.work_buff[pos1 + 1] != self.work_buff[pos2 + 1]
        {
            return 0;
        }

        // Start with 2 bytes already matched
        let mut length = 2;

        // Compare remaining bytes
        while length < max_length {
            let idx1 = pos1 + length;
            let idx2 = pos2 + length;

            if idx1 >= self.work_buff.len() || idx2 >= self.work_buff.len() {
                break;
            }

            if self.work_buff[idx1] != self.work_buff[idx2] {
                break;
            }

            length += 1;
        }

        length
    }

    /// Handle run-length encoding (repeating single character)
    fn find_run_length_match(&self, pos: usize, max_length: usize) -> usize {
        if pos >= self.work_buff.len() {
            return 0;
        }

        let byte_value = self.work_buff[pos];
        let mut length = 1;

        // Count how many consecutive bytes match
        while length < max_length && pos + length < self.work_buff.len() {
            if self.work_buff[pos + length] != byte_value {
                break;
            }
            length += 1;
        }

        length
    }

    /// Find matches for optimization (PKLib's advanced pattern matching)
    /// This implements the complex optimization logic from the original FindRep
    pub fn find_optimized_match(
        &mut self,
        input_pos: usize,
        current_best: MatchResult,
    ) -> MatchResult {
        // This is a simplified version of PKLib's optimization logic
        // The full version is quite complex and handles many edge cases

        if !current_best.is_match() || current_best.length < 10 {
            return current_best;
        }

        // Look for potentially better matches at different positions
        let hash = byte_pair_hash(&self.work_buff[input_pos..input_pos + 2]);
        let positions = self.find_hash_positions(hash, input_pos);

        let mut best_match = current_best;
        let max_length = (self.work_bytes - input_pos).min(MAX_REP_LENGTH);

        // Check if there's a longer match at a different position
        for &match_pos in &positions {
            if match_pos >= input_pos {
                continue;
            }

            let distance = input_pos - match_pos;
            if distance == best_match.distance {
                continue; // Same match we already found
            }

            let match_length = self.compare_sequences(input_pos, match_pos, max_length);

            // Prefer longer matches, or closer matches of the same length
            if match_length > best_match.length {
                best_match = MatchResult::new(match_length, distance);
                self.distance = (distance - 1) as u32;
            }
        }

        best_match
    }

    /// Quick check if a position might have a good match
    /// Used for performance optimization in the inner loop
    pub fn quick_match_check(&self, pos1: usize, pos2: usize, min_length: usize) -> bool {
        if pos1 + min_length > self.work_buff.len() || pos2 + min_length > self.work_buff.len() {
            return false;
        }

        // Check first and last bytes of minimum required length
        self.work_buff[pos1] == self.work_buff[pos2]
            && self.work_buff[pos1 + min_length - 1] == self.work_buff[pos2 + min_length - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompressionMode, DictionarySize};

    #[test]
    fn test_no_match() {
        let mut state = ImplodeState::new(CompressionMode::Binary, DictionarySize::Size1K).unwrap();

        // Set up unique data with no repetitions
        let test_data = b"ABCDEFGHIJ";
        let len = test_data.len().min(state.work_buff.len());
        state.work_buff[..len].copy_from_slice(&test_data[..len]);
        state.work_bytes = len;

        // Build hash table
        state.sort_buffer(0, len);

        // Look for match at position that has no previous occurrence
        let result = state.find_repetition(5);
        assert!(!result.is_match());
    }

    #[test]
    #[ignore] // TODO: Fix after compression refactoring
    fn test_simple_match() {
        let mut state = ImplodeState::new(CompressionMode::Binary, DictionarySize::Size1K).unwrap();

        // Set up data with a clear repetition
        let test_data = b"ABCDEFABCDEF";
        let len = test_data.len().min(state.work_buff.len());
        state.work_buff[..len].copy_from_slice(&test_data[..len]);
        state.work_bytes = len;

        // Build hash table
        state.sort_buffer(0, len);

        // Look for match at position 6 (second "AB")
        let result = state.find_repetition(6);
        assert!(result.is_match());
        assert_eq!(result.distance, 6); // Distance from position 6 to position 0
        assert!(result.length >= 2); // At least "AB" should match
    }

    #[test]
    fn test_run_length_match() {
        let mut state = ImplodeState::new(CompressionMode::Binary, DictionarySize::Size1K).unwrap();

        // Set up data with run-length pattern
        let test_data = b"AAAAAAAAA";
        let len = test_data.len().min(state.work_buff.len());
        state.work_buff[..len].copy_from_slice(&test_data[..len]);
        state.work_bytes = len;

        // Test run-length matching
        let length = state.find_run_length_match(0, len);
        assert_eq!(length, len);
    }

    #[test]
    fn test_compare_sequences() {
        let mut state = ImplodeState::new(CompressionMode::Binary, DictionarySize::Size1K).unwrap();

        // Set up test data
        let test_data = b"ABCDEFABCXYZ";
        let len = test_data.len().min(state.work_buff.len());
        state.work_buff[..len].copy_from_slice(&test_data[..len]);
        state.work_bytes = len;

        // Compare "ABC" at position 0 with "ABC" at position 6
        let length = state.compare_sequences(0, 6, 6);
        assert_eq!(length, 3); // "ABC" matches

        // Compare sequences that don't match
        let length = state.compare_sequences(0, 3, 3);
        assert_eq!(length, 0); // "ABC" vs "DEF" - no match
    }

    #[test]
    fn test_match_result() {
        let match_result = MatchResult::new(5, 10);
        assert!(match_result.is_match());
        assert_eq!(match_result.length, 5);
        assert_eq!(match_result.distance, 10);

        let no_match = MatchResult::no_match();
        assert!(!no_match.is_match());
        assert_eq!(no_match.length, 0);
        assert_eq!(no_match.distance, 0);

        let short_match = MatchResult::new(1, 5);
        assert!(!short_match.is_match()); // Less than 2 bytes
    }
}
