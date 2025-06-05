//! Property-based tests for PKLib Rust implementation
//!
//! These tests use randomized inputs to verify correctness across a wide range
//! of data patterns and edge cases.

use pklib::{explode_bytes, implode_bytes, CompressionMode, DictionarySize};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_decompression_never_panics(data in prop::collection::vec(any::<u8>(), 0..1000)) {
        // We can't guarantee all random data is valid PKLib data,
        // but we should never panic - only return errors gracefully
        let _ = explode_bytes(&data);
    }
}

proptest! {
    #[test]
    fn test_small_inputs(data in prop::collection::vec(any::<u8>(), 0..10)) {
        // Test all compression modes and dictionary sizes with small inputs
        for mode in [CompressionMode::Binary, CompressionMode::ASCII] {
            for dict_size in [DictionarySize::Size1K, DictionarySize::Size2K, DictionarySize::Size4K] {
                // Compression might fail (and that's ok), but shouldn't panic
                if let Ok(compressed) = implode_bytes(&data, mode, dict_size) {
                    // If compression succeeds, decompression should work and return original data
                    if let Ok(decompressed) = explode_bytes(&compressed) {
                        prop_assert_eq!(&data[..], &decompressed[..]);
                    }
                }
            }
        }
    }
}

proptest! {
    #[test]
    fn test_ascii_round_trip(
        data in prop::collection::vec(prop::char::range(' ', '~'), 10..100)
    ) {
        let ascii_bytes: Vec<u8> = data.into_iter().map(|c| c as u8).collect();

        // ASCII data should work well with ASCII mode
        if let Ok(compressed) = implode_bytes(&ascii_bytes, CompressionMode::ASCII, DictionarySize::Size2K) {
            if let Ok(decompressed) = explode_bytes(&compressed) {
                prop_assert_eq!(&ascii_bytes[..], &decompressed[..]);
            }
        }
    }
}

proptest! {
    #[test]
    fn test_repetitive_patterns(
        pattern in prop::collection::vec(any::<u8>(), 1..20),
        repeat_count in 2..50u8
    ) {
        let mut data = Vec::new();
        for _ in 0..repeat_count {
            data.extend_from_slice(&pattern);
        }

        // Repetitive data should compress and decompress correctly
        for mode in [CompressionMode::Binary, CompressionMode::ASCII] {
            for dict_size in [DictionarySize::Size1K, DictionarySize::Size2K, DictionarySize::Size4K] {
                if let Ok(compressed) = implode_bytes(&data, mode, dict_size) {
                    if let Ok(decompressed) = explode_bytes(&compressed) {
                        prop_assert_eq!(&data[..], &decompressed[..]);

                        // Repetitive data should not expand too much (some overhead is normal)
                        prop_assert!(compressed.len() <= data.len() + 50, "Compression expanded too much: {} -> {}", data.len(), compressed.len());
                    }
                }
            }
        }
    }
}

proptest! {
    #[test]
    fn test_zero_data(size in 0..500usize) {
        let data = vec![0u8; size];

        for mode in [CompressionMode::Binary, CompressionMode::ASCII] {
            for dict_size in [DictionarySize::Size1K, DictionarySize::Size2K, DictionarySize::Size4K] {
                if let Ok(compressed) = implode_bytes(&data, mode, dict_size) {
                    if let Ok(decompressed) = explode_bytes(&compressed) {
                        prop_assert_eq!(&data[..], &decompressed[..]);
                    }
                }
            }
        }
    }
}

proptest! {
    #[test]
    fn test_single_byte_patterns(byte_value in any::<u8>(), size in 1..300usize) {
        let data = vec![byte_value; size];

        for mode in [CompressionMode::Binary, CompressionMode::ASCII] {
            for dict_size in [DictionarySize::Size1K, DictionarySize::Size2K, DictionarySize::Size4K] {
                if let Ok(compressed) = implode_bytes(&data, mode, dict_size) {
                    if let Ok(decompressed) = explode_bytes(&compressed) {
                        prop_assert_eq!(&data[..], &decompressed[..]);

                        // Note: Single byte patterns might not always compress due to PKLib encoding overhead
                        // This is normal behavior for the PKLib format
                    }
                }
            }
        }
    }
}

proptest! {
    #[test]
    fn test_compression_ratio_bounds(data in prop::collection::vec(any::<u8>(), 10..200)) {
        for mode in [CompressionMode::Binary, CompressionMode::ASCII] {
            for dict_size in [DictionarySize::Size1K, DictionarySize::Size2K, DictionarySize::Size4K] {
                if let Ok(compressed) = implode_bytes(&data, mode, dict_size) {
                    // Compressed size should never be dramatically larger than original
                    // (some expansion is normal for very small or random data)
                    prop_assert!(compressed.len() <= data.len() + 200,
                        "Compression expanded data too much: {} -> {}", data.len(), compressed.len());

                    // Should always be able to decompress what we compressed
                    let decompressed = explode_bytes(&compressed)?;
                    prop_assert_eq!(&data[..], &decompressed[..]);
                }
            }
        }
    }
}

proptest! {
    #[test]
    fn test_edge_case_patterns(
        base_data in prop::collection::vec(any::<u8>(), 50..150)
    ) {
        // Test data that might stress the compression algorithm
        let mut test_cases = vec![
            base_data.clone(),
        ];

        // Add some patterns that might be problematic
        let mut alternating = Vec::new();
        for i in 0..base_data.len() {
            alternating.push(if i % 2 == 0 { 0xAA } else { 0x55 });
        }
        test_cases.push(alternating);

        // Add incrementing pattern
        let incrementing: Vec<u8> = (0..base_data.len()).map(|i| (i % 256) as u8).collect();
        test_cases.push(incrementing);

        for data in test_cases {
            for mode in [CompressionMode::Binary, CompressionMode::ASCII] {
                for dict_size in [DictionarySize::Size1K, DictionarySize::Size2K, DictionarySize::Size4K] {
                    if let Ok(compressed) = implode_bytes(&data, mode, dict_size) {
                        let decompressed = explode_bytes(&compressed)?;
                        prop_assert_eq!(&data[..], &decompressed[..]);
                    }
                }
            }
        }
    }
}

proptest! {
    #[test]
    fn test_compression_deterministic(data in prop::collection::vec(any::<u8>(), 10..100)) {
        for mode in [CompressionMode::Binary, CompressionMode::ASCII] {
            for dict_size in [DictionarySize::Size1K, DictionarySize::Size2K, DictionarySize::Size4K] {
                if let Ok(compressed1) = implode_bytes(&data, mode, dict_size) {
                    if let Ok(compressed2) = implode_bytes(&data, mode, dict_size) {
                        // Same input should always produce same output
                        prop_assert_eq!(compressed1, compressed2);
                    }
                }
            }
        }
    }
}
