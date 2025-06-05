use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pklib::{explode_bytes, implode_bytes, CompressionMode, DictionarySize};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Custom allocator to track memory usage
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK_ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);
        if !ret.is_null() {
            let old_size = ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
            let new_size = old_size + layout.size();
            let mut peak = PEAK_ALLOCATED.load(Ordering::SeqCst);
            while new_size > peak {
                match PEAK_ALLOCATED.compare_exchange_weak(
                    peak,
                    new_size,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(p) => peak = p,
                }
            }
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn reset_memory_tracking() {
    ALLOCATED.store(0, Ordering::SeqCst);
    PEAK_ALLOCATED.store(0, Ordering::SeqCst);
}

fn get_peak_memory() -> usize {
    PEAK_ALLOCATED.load(Ordering::SeqCst)
}

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
        "repetitive" => {
            vec![b'A'; size]
        }
        _ => panic!("Unknown pattern: {}", pattern),
    }
}

fn compression_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_memory");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    // Test memory usage for different sizes
    for size in [10240, 102400, 1048576, 10485760].iter() {
        let size_label = match *size {
            10240 => "10KB",
            102400 => "100KB",
            1048576 => "1MB",
            10485760 => "10MB",
            _ => "unknown",
        };

        let data = generate_test_data(*size, "text");

        for dict_size in [
            DictionarySize::Size1K,
            DictionarySize::Size2K,
            DictionarySize::Size4K,
        ]
        .iter()
        {
            let dict_str = match dict_size {
                DictionarySize::Size1K => "1KB",
                DictionarySize::Size2K => "2KB",
                DictionarySize::Size4K => "4KB",
            };

            let benchmark_id =
                BenchmarkId::from_parameter(format!("{}/dict_{}", size_label, dict_str));

            group.bench_with_input(benchmark_id, &data, |b, data| {
                b.iter_custom(|iters| {
                    let mut total_duration = Duration::new(0, 0);
                    let mut _peak_memory_sum = 0;

                    for _ in 0..iters {
                        reset_memory_tracking();

                        let start = std::time::Instant::now();
                        let _compressed = implode_bytes(
                            black_box(data),
                            black_box(CompressionMode::Binary),
                            black_box(*dict_size),
                        )
                        .expect("Compression failed");
                        let duration = start.elapsed();

                        total_duration += duration;
                        _peak_memory_sum += get_peak_memory();
                    }

                    // Return average duration
                    total_duration / iters as u32
                });
            });
        }
    }

    group.finish();
}

fn decompression_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("decompression_memory");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    // Test memory usage for different compressed data sizes
    for size in [10240, 102400, 1048576, 10485760].iter() {
        let size_label = match *size {
            10240 => "10KB",
            102400 => "100KB",
            1048576 => "1MB",
            10485760 => "10MB",
            _ => "unknown",
        };

        let original_data = generate_test_data(*size, "text");
        let compressed_data = implode_bytes(
            &original_data,
            CompressionMode::Binary,
            DictionarySize::Size4K,
        )
        .expect("Compression failed");

        let benchmark_id = BenchmarkId::from_parameter(size_label);

        group.bench_with_input(benchmark_id, &compressed_data, |b, data| {
            b.iter_custom(|iters| {
                let mut total_duration = Duration::new(0, 0);
                let mut _peak_memory_sum = 0;

                for _ in 0..iters {
                    reset_memory_tracking();

                    let start = std::time::Instant::now();
                    let _decompressed =
                        explode_bytes(black_box(data)).expect("Decompression failed");
                    let duration = start.elapsed();

                    total_duration += duration;
                    _peak_memory_sum += get_peak_memory();
                }

                // Return average duration
                total_duration / iters as u32
            });
        });
    }

    group.finish();
}

fn round_trip_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_trip_memory");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(25);

    // Test complete round-trip memory usage
    for size in [102400, 1048576, 10485760].iter() {
        let size_label = match *size {
            102400 => "100KB",
            1048576 => "1MB",
            10485760 => "10MB",
            _ => "unknown",
        };

        let data = generate_test_data(*size, "text");

        let benchmark_id = BenchmarkId::from_parameter(size_label);

        group.bench_with_input(benchmark_id, &data, |b, data| {
            b.iter_custom(|iters| {
                let mut total_duration = Duration::new(0, 0);
                let mut _peak_memory_sum = 0;

                for _ in 0..iters {
                    reset_memory_tracking();

                    let start = std::time::Instant::now();

                    // Compress
                    let compressed = implode_bytes(
                        black_box(data),
                        black_box(CompressionMode::Binary),
                        black_box(DictionarySize::Size4K),
                    )
                    .expect("Compression failed");

                    // Decompress
                    let _decompressed =
                        explode_bytes(black_box(&compressed)).expect("Decompression failed");

                    let duration = start.elapsed();

                    total_duration += duration;
                    _peak_memory_sum += get_peak_memory();
                }

                // Return average duration
                total_duration / iters as u32
            });
        });
    }

    group.finish();
}

fn memory_efficiency_by_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");
    group.measurement_time(Duration::from_secs(8));

    let size = 1048576; // 1MB

    let patterns = vec![
        ("highly_repetitive", vec![b'X'; size]),
        ("moderately_repetitive", {
            let mut data = Vec::with_capacity(size);
            for i in 0..size {
                data.push((i % 10) as u8 + b'0');
            }
            data
        }),
        ("low_repetition", {
            (0..size).map(|i| ((i * 17) % 256) as u8).collect()
        }),
    ];

    for (pattern_name, data) in patterns {
        let benchmark_id = BenchmarkId::from_parameter(pattern_name);

        group.bench_with_input(benchmark_id, &data, |b, data| {
            b.iter_custom(|iters| {
                let mut total_duration = Duration::new(0, 0);
                let mut _compression_memory_sum = 0;
                let mut _decompression_memory_sum = 0;

                for _ in 0..iters {
                    // Measure compression memory
                    reset_memory_tracking();
                    let start = std::time::Instant::now();

                    let compressed = implode_bytes(
                        black_box(data),
                        black_box(CompressionMode::Binary),
                        black_box(DictionarySize::Size4K),
                    )
                    .expect("Compression failed");

                    _compression_memory_sum += get_peak_memory();

                    // Measure decompression memory
                    reset_memory_tracking();

                    let _decompressed =
                        explode_bytes(black_box(&compressed)).expect("Decompression failed");

                    let duration = start.elapsed();
                    _decompression_memory_sum += get_peak_memory();

                    total_duration += duration;
                }

                // Return average duration
                total_duration / iters as u32
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    compression_memory_usage,
    decompression_memory_usage,
    round_trip_memory_usage,
    memory_efficiency_by_pattern
);
criterion_main!(benches);
