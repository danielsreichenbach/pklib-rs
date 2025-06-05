use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use pklib::{implode_bytes, CompressionMode, DictionarySize};
use std::hint::black_box;
use std::time::Duration;

fn generate_test_data(size: usize, pattern: &str) -> Vec<u8> {
    match pattern {
        "text" => {
            // Generate Lorem ipsum style text data
            let base = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";
            let mut data = Vec::with_capacity(size);
            while data.len() < size {
                data.extend_from_slice(base);
            }
            data.truncate(size);
            data
        }
        "binary" => {
            // Generate binary data with some patterns
            (0..size).map(|i| ((i * 17 + 11) % 256) as u8).collect()
        }
        "repetitive" => {
            // Highly repetitive data that compresses well
            let pattern = b"ABCDEFGHIJ";
            let mut data = Vec::with_capacity(size);
            while data.len() < size {
                data.extend_from_slice(pattern);
            }
            data.truncate(size);
            data
        }
        "random" => {
            // Pseudo-random data that compresses poorly
            (0..size)
                .map(|i| {
                    let x = i as u32;
                    ((x.wrapping_mul(1664525).wrapping_add(1013904223)) % 256) as u8
                })
                .collect()
        }
        _ => panic!("Unknown pattern: {}", pattern),
    }
}

fn compression_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_throughput");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

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
        for pattern in ["text", "binary", "repetitive", "random"].iter() {
            let data = generate_test_data(*size, pattern);

            // Test different compression modes and dictionary sizes
            for mode in [CompressionMode::Binary, CompressionMode::ASCII].iter() {
                for dict_size in [
                    DictionarySize::Size1K,
                    DictionarySize::Size2K,
                    DictionarySize::Size4K,
                ]
                .iter()
                {
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

                    group.throughput(Throughput::Bytes(*size as u64));
                    group.bench_with_input(benchmark_id, &data, |b, data| {
                        b.iter(|| {
                            implode_bytes(black_box(data), black_box(*mode), black_box(*dict_size))
                                .expect("Compression failed")
                        });
                    });
                }
            }
        }
    }

    group.finish();
}

fn compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");
    group.measurement_time(Duration::from_secs(5));

    // Test compression ratios for different data types
    let test_sizes = vec![10240, 102400]; // 10KB and 100KB

    for size in test_sizes {
        for pattern in ["text", "binary", "repetitive", "random"].iter() {
            let data = generate_test_data(size, pattern);

            for mode in [CompressionMode::Binary, CompressionMode::ASCII].iter() {
                let dict_size = DictionarySize::Size4K; // Use 4KB for ratio tests

                let mode_str = match mode {
                    CompressionMode::Binary => "binary",
                    CompressionMode::ASCII => "ascii",
                };

                let benchmark_id =
                    BenchmarkId::from_parameter(format!("{}/{}/{}", size, pattern, mode_str));

                group.bench_with_input(benchmark_id, &data, |b, data| {
                    b.iter_batched(
                        || data.clone(),
                        |data| {
                            let compressed = implode_bytes(
                                black_box(&data),
                                black_box(*mode),
                                black_box(dict_size),
                            )
                            .expect("Compression failed");

                            // Return compression ratio
                            let ratio = compressed.len() as f64 / data.len() as f64;
                            black_box(ratio)
                        },
                        BatchSize::SmallInput,
                    );
                });
            }
        }
    }

    group.finish();
}

fn large_file_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_file_compression");
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
        let data = generate_test_data(*size, "text");

        let mode = CompressionMode::Binary;
        let dict_size = DictionarySize::Size4K;

        let benchmark_id = BenchmarkId::from_parameter(size_label);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(benchmark_id, &data, |b, data| {
            b.iter(|| {
                implode_bytes(black_box(data), black_box(mode), black_box(dict_size))
                    .expect("Compression failed")
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    compression_throughput,
    compression_ratio,
    large_file_compression
);
criterion_main!(benches);
