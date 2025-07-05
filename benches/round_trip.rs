use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pklib::{explode_bytes, implode_bytes, CompressionMode, DictionarySize};
use std::hint::black_box;
use std::time::Duration;

fn generate_test_data(size: usize, pattern: &str) -> Vec<u8> {
    match pattern {
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
        "json" => {
            // Simulate JSON-like data
            let template = br#"{"id":123,"name":"Example","values":[1,2,3,4,5],"active":true}"#;
            let mut data = Vec::with_capacity(size);
            while data.len() < size {
                data.extend_from_slice(template);
                data.push(b',');
            }
            data.truncate(size);
            data
        }
        _ => panic!("Unknown pattern: {pattern}"),
    }
}

fn round_trip_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_trip_throughput");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(50);

    // Test different file sizes
    for size in [1024, 10240, 102400, 1048576].iter() {
        let size_label = match *size {
            1024 => "1KB",
            10240 => "10KB",
            102400 => "100KB",
            1048576 => "1MB",
            _ => "unknown",
        };

        // Test different data patterns
        for pattern in ["text", "binary", "repetitive", "json"].iter() {
            let data = generate_test_data(*size, pattern);

            // Test different configurations
            for mode in [CompressionMode::Binary, CompressionMode::ASCII].iter() {
                for dict_size in [DictionarySize::Size1K, DictionarySize::Size4K].iter() {
                    let mode_str = match mode {
                        CompressionMode::Binary => "binary",
                        CompressionMode::ASCII => "ascii",
                    };
                    let dict_str = match dict_size {
                        DictionarySize::Size1K => "1KB",
                        DictionarySize::Size4K => "4KB",
                        _ => "other",
                    };

                    let benchmark_id = BenchmarkId::from_parameter(format!(
                        "{size_label}/{pattern}/{mode_str}/{dict_str}"
                    ));

                    group.throughput(Throughput::Bytes(*size as u64));
                    group.bench_with_input(benchmark_id, &data, |b, data| {
                        b.iter(|| {
                            // Compress
                            let compressed = implode_bytes(
                                black_box(data),
                                black_box(*mode),
                                black_box(*dict_size),
                            )
                            .expect("Compression failed");

                            // Decompress
                            let decompressed = explode_bytes(black_box(&compressed))
                                .expect("Decompression failed");

                            // Verify round-trip integrity
                            assert_eq!(data.len(), decompressed.len());
                            decompressed
                        });
                    });
                }
            }
        }
    }

    group.finish();
}

fn round_trip_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_trip_memory");
    group.measurement_time(Duration::from_secs(10));

    // Focus on memory efficiency for different sizes
    let test_configs = vec![
        (10240, "10KB"),    // Small
        (1048576, "1MB"),   // Medium
        (10485760, "10MB"), // Large
    ];

    for (size, label) in test_configs {
        let data = generate_test_data(size, "text");
        let mode = CompressionMode::Binary;
        let dict_size = DictionarySize::Size4K;

        let benchmark_id = BenchmarkId::from_parameter(label);

        group.bench_with_input(benchmark_id, &data, |b, data| {
            b.iter(|| {
                // Measure complete round-trip with intermediate storage
                let compressed =
                    implode_bytes(black_box(data), black_box(mode), black_box(dict_size))
                        .expect("Compression failed");

                let compressed_size = compressed.len();

                let decompressed =
                    explode_bytes(black_box(&compressed)).expect("Decompression failed");

                // Return tuple to prevent optimization
                (decompressed, compressed_size)
            });
        });
    }

    group.finish();
}

fn round_trip_data_integrity(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_trip_integrity");
    group.measurement_time(Duration::from_secs(5));

    // Test data integrity for edge cases
    let edge_cases = vec![
        ("empty", vec![]),
        ("single_byte", vec![b'X']),
        ("min_match", vec![b'A', b'B', b'C']),
        ("boundary_4KB", vec![b'Z'; 4096]),
        ("boundary_4KB_plus_1", vec![b'Y'; 4097]),
        ("max_repetition", vec![b'R'; 516]),
        (
            "alternating",
            (0..1000)
                .map(|i| if i % 2 == 0 { b'A' } else { b'B' })
                .collect(),
        ),
    ];

    for (name, data) in edge_cases {
        let mode = CompressionMode::Binary;
        let dict_size = DictionarySize::Size4K;

        let benchmark_id = BenchmarkId::from_parameter(name);

        group.bench_with_input(benchmark_id, &data, |b, data| {
            b.iter(|| {
                let compressed =
                    implode_bytes(black_box(data), black_box(mode), black_box(dict_size))
                        .expect("Compression failed");

                let decompressed =
                    explode_bytes(black_box(&compressed)).expect("Decompression failed");

                // Verify exact match
                assert_eq!(data, &decompressed);
                decompressed
            });
        });
    }

    group.finish();
}

fn round_trip_compression_effectiveness(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_trip_effectiveness");
    group.measurement_time(Duration::from_secs(8));

    // Measure compression effectiveness across different scenarios
    let size = 102400; // 100KB

    struct Scenario {
        name: &'static str,
        data_gen: fn(usize) -> Vec<u8>,
    }

    let scenarios = vec![
        Scenario {
            name: "highly_compressible",
            data_gen: |size| vec![b'A'; size],
        },
        Scenario {
            name: "source_code",
            data_gen: |size| {
                let template = b"fn process_data(input: &[u8]) -> Result<Vec<u8>, Error> {\n    let mut output = Vec::new();\n    // Process the data\n    Ok(output)\n}\n";
                let mut data = Vec::with_capacity(size);
                while data.len() < size {
                    data.extend_from_slice(template);
                }
                data.truncate(size);
                data
            },
        },
        Scenario {
            name: "mixed_entropy",
            data_gen: |size| {
                (0..size)
                    .map(|i| {
                        if i % 100 < 50 {
                            b'X' // Repetitive section
                        } else {
                            ((i * 7) % 256) as u8 // Variable section
                        }
                    })
                    .collect()
            },
        },
    ];

    for scenario in scenarios {
        let data = (scenario.data_gen)(size);

        for mode in [CompressionMode::Binary, CompressionMode::ASCII].iter() {
            let dict_size = DictionarySize::Size4K;

            let mode_str = match mode {
                CompressionMode::Binary => "binary",
                CompressionMode::ASCII => "ascii",
            };

            let benchmark_id =
                BenchmarkId::from_parameter(format!("{}/{}", scenario.name, mode_str));

            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(benchmark_id, &data, |b, data| {
                b.iter(|| {
                    let compressed =
                        implode_bytes(black_box(data), black_box(*mode), black_box(dict_size))
                            .expect("Compression failed");

                    let compression_ratio = compressed.len() as f64 / data.len() as f64;

                    let decompressed =
                        explode_bytes(black_box(&compressed)).expect("Decompression failed");

                    (decompressed, compression_ratio)
                });
            });
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    round_trip_throughput,
    round_trip_memory_efficiency,
    round_trip_data_integrity,
    round_trip_compression_effectiveness
);
criterion_main!(benches);
