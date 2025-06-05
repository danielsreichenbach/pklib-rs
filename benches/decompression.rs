use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pklib::{explode_bytes, implode_bytes, CompressionMode, DictionarySize};
use std::hint::black_box;
use std::time::Duration;

fn generate_compressed_data(
    size: usize,
    pattern: &str,
    mode: CompressionMode,
    dict_size: DictionarySize,
) -> Vec<u8> {
    let original = match pattern {
        "text" => {
            let base = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";
            let mut data = Vec::with_capacity(size);
            while data.len() < size {
                data.extend_from_slice(base);
            }
            data.truncate(size);
            data
        }
        "binary" => (0..size).map(|i| ((i * 17 + 11) % 256) as u8).collect(),
        "repetitive" => {
            let pattern = b"ABCDEFGHIJ";
            let mut data = Vec::with_capacity(size);
            while data.len() < size {
                data.extend_from_slice(pattern);
            }
            data.truncate(size);
            data
        }
        "random" => (0..size)
            .map(|i| {
                let x = i as u32;
                ((x.wrapping_mul(1664525).wrapping_add(1013904223)) % 256) as u8
            })
            .collect(),
        _ => panic!("Unknown pattern: {}", pattern),
    };

    implode_bytes(&original, mode, dict_size).expect("Compression failed")
}

fn decompression_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("decompression_throughput");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    // Test different original file sizes
    for size in [1024, 10240, 102400, 1048576].iter() {
        let size_label = match *size {
            1024 => "1KB",
            10240 => "10KB",
            102400 => "100KB",
            1048576 => "1MB",
            _ => "unknown",
        };

        // Test different data patterns
        for pattern in ["text", "binary", "repetitive", "random"].iter() {
            // Test different compression modes and dictionary sizes
            for mode in [CompressionMode::Binary, CompressionMode::ASCII].iter() {
                for dict_size in [
                    DictionarySize::Size1K,
                    DictionarySize::Size2K,
                    DictionarySize::Size4K,
                ]
                .iter()
                {
                    let compressed_data =
                        generate_compressed_data(*size, pattern, *mode, *dict_size);

                    let mode_str = match mode {
                        CompressionMode::Binary => "binary",
                        CompressionMode::ASCII => "ascii",
                    };
                    let dict_str = match dict_size {
                        DictionarySize::Size1K => "1KB",
                        DictionarySize::Size2K => "2KB",
                        DictionarySize::Size4K => "4KB",
                    };

                    let benchmark_id = BenchmarkId::from_parameter(format!(
                        "{}/{}/{}/{}",
                        size_label, pattern, mode_str, dict_str
                    ));

                    // Throughput is based on uncompressed size
                    group.throughput(Throughput::Bytes(*size as u64));
                    group.bench_with_input(benchmark_id, &compressed_data, |b, data| {
                        b.iter(|| explode_bytes(black_box(data)).expect("Decompression failed"));
                    });
                }
            }
        }
    }

    group.finish();
}

fn decompression_by_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("decompression_by_ratio");
    group.measurement_time(Duration::from_secs(5));

    // Test decompression speed for different compression ratios
    let original_size = 102400; // 100KB

    struct TestCase {
        pattern: &'static str,
        expected_ratio: &'static str,
    }

    let test_cases = vec![
        TestCase {
            pattern: "repetitive",
            expected_ratio: "high_compression",
        },
        TestCase {
            pattern: "text",
            expected_ratio: "medium_compression",
        },
        TestCase {
            pattern: "binary",
            expected_ratio: "low_compression",
        },
        TestCase {
            pattern: "random",
            expected_ratio: "minimal_compression",
        },
    ];

    for test_case in test_cases {
        let mode = CompressionMode::Binary;
        let dict_size = DictionarySize::Size4K;

        let compressed_data =
            generate_compressed_data(original_size, test_case.pattern, mode, dict_size);
        let actual_ratio = compressed_data.len() as f64 / original_size as f64;

        let benchmark_id = BenchmarkId::from_parameter(format!(
            "{} (ratio: {:.2})",
            test_case.expected_ratio, actual_ratio
        ));

        group.throughput(Throughput::Bytes(original_size as u64));
        group.bench_with_input(benchmark_id, &compressed_data, |b, data| {
            b.iter(|| explode_bytes(black_box(data)).expect("Decompression failed"));
        });
    }

    group.finish();
}

fn large_file_decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_file_decompression");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    // Test larger files
    for size in [10485760, 104857600].iter() {
        // 10MB and 100MB
        let size_label = match *size {
            10485760 => "10MB",
            104857600 => "100MB",
            _ => "unknown",
        };

        // Only test with text pattern for large files
        let mode = CompressionMode::Binary;
        let dict_size = DictionarySize::Size4K;
        let compressed_data = generate_compressed_data(*size, "text", mode, dict_size);

        let benchmark_id = BenchmarkId::from_parameter(size_label);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(benchmark_id, &compressed_data, |b, data| {
            b.iter(|| explode_bytes(black_box(data)).expect("Decompression failed"));
        });
    }

    group.finish();
}

fn decompression_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("decompression_edge_cases");
    group.measurement_time(Duration::from_secs(3));

    // Test edge cases
    let test_cases = vec![
        ("empty", vec![]),
        ("single_byte", vec![b'A']),
        ("min_match", vec![b'A', b'B', b'C']), // Minimum match length is 3
        ("max_repetition", vec![b'X'; 516]),   // MAX_REP_LENGTH
    ];

    for (name, data) in test_cases {
        let mode = CompressionMode::Binary;
        let dict_size = DictionarySize::Size4K;

        let compressed = implode_bytes(&data, mode, dict_size).expect("Compression failed");

        let benchmark_id = BenchmarkId::from_parameter(name);

        group.bench_with_input(benchmark_id, &compressed, |b, data| {
            b.iter(|| explode_bytes(black_box(data)).expect("Decompression failed"));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    decompression_throughput,
    decompression_by_ratio,
    large_file_decompression,
    decompression_edge_cases
);
criterion_main!(benches);
