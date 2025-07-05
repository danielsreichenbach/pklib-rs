use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pklib::{explode_bytes, implode_bytes, CompressionMode, DictionarySize};
use std::hint::black_box;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn generate_test_files(count: usize, size_per_file: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| {
            // Generate slightly different data for each file
            let base = format!("File {i} content: Lorem ipsum dolor sit amet. ");
            let mut data = Vec::with_capacity(size_per_file);
            while data.len() < size_per_file {
                data.extend_from_slice(base.as_bytes());
            }
            data.truncate(size_per_file);
            data
        })
        .collect()
}

fn generate_large_file(size: usize) -> Vec<u8> {
    let pattern = b"The quick brown fox jumps over the lazy dog. ";
    let mut data = Vec::with_capacity(size);
    while data.len() < size {
        data.extend_from_slice(pattern);
    }
    data.truncate(size);
    data
}

/// Benchmark multiple files processed in parallel
fn parallel_file_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_compression");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(20);

    // Test different file counts and sizes
    let test_cases = vec![
        (10, 102400), // 10 files × 100KB each
        (100, 10240), // 100 files × 10KB each
        (1000, 1024), // 1000 files × 1KB each
    ];

    for (file_count, file_size) in test_cases {
        let files = generate_test_files(file_count, file_size);
        let total_size = file_count * file_size;

        // Test with different thread counts
        for thread_count in [1, 2, 4, 8].iter() {
            let benchmark_id = BenchmarkId::from_parameter(format!(
                "{}files_{}KB_{}threads",
                file_count,
                file_size / 1024,
                thread_count
            ));

            group.throughput(Throughput::Bytes(total_size as u64));
            group.bench_with_input(benchmark_id, &files, |b, files: &Vec<Vec<u8>>| {
                b.iter(|| {
                    let files = files.clone();
                    let thread_count = *thread_count;

                    // Create work queue
                    let work_queue = Arc::new(Mutex::new(
                        files.into_iter().enumerate().collect::<Vec<_>>(),
                    ));
                    let results = Arc::new(Mutex::new(Vec::with_capacity(file_count)));

                    // Spawn worker threads
                    let mut handles = vec![];
                    for _ in 0..thread_count {
                        let queue = Arc::clone(&work_queue);
                        let results = Arc::clone(&results);

                        let handle = thread::spawn(move || {
                            loop {
                                // Get next file to process
                                let work_item = {
                                    let mut queue = queue.lock().unwrap();
                                    queue.pop()
                                };

                                let (idx, data) = match work_item {
                                    Some(item) => item,
                                    None => break,
                                };

                                // Compress the file
                                let compressed = implode_bytes(
                                    &data,
                                    CompressionMode::Binary,
                                    DictionarySize::Size4K,
                                )
                                .expect("Compression failed");

                                // Store result
                                let mut results = results.lock().unwrap();
                                results.push((idx, compressed));
                            }
                        });

                        handles.push(handle);
                    }

                    // Wait for all threads to complete
                    for handle in handles {
                        handle.join().unwrap();
                    }

                    // Return sorted results
                    let mut results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
                    results.sort_by_key(|(idx, _)| *idx);
                    results
                });
            });
        }
    }

    group.finish();
}

/// Benchmark single file split into chunks for parallel processing
fn parallel_chunk_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_chunk_compression");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(20);

    // Test different file sizes
    for file_size in [1048576, 10485760, 104857600].iter() {
        // 1MB, 10MB, 100MB
        let size_label = match *file_size {
            1048576 => "1MB",
            10485760 => "10MB",
            104857600 => "100MB",
            _ => "unknown",
        };

        let data = generate_large_file(*file_size);

        // Test with different chunk sizes and thread counts
        for chunk_size in [65536, 262144, 1048576].iter() {
            // 64KB, 256KB, 1MB chunks
            let chunk_label = match *chunk_size {
                65536 => "64KB",
                262144 => "256KB",
                1048576 => "1MB",
                _ => "unknown",
            };

            for thread_count in [1, 2, 4, 8].iter() {
                let benchmark_id = BenchmarkId::from_parameter(format!(
                    "{size_label}_{chunk_label}chunks_{thread_count}threads"
                ));

                group.throughput(Throughput::Bytes(*file_size as u64));
                group.bench_with_input(benchmark_id, &data, |b, data| {
                    b.iter(|| {
                        let chunks: Vec<Vec<u8>> = data
                            .chunks(*chunk_size)
                            .map(|chunk| chunk.to_vec())
                            .collect();

                        let thread_count = *thread_count;
                        let work_queue = Arc::new(Mutex::new(
                            chunks.into_iter().enumerate().collect::<Vec<_>>(),
                        ));
                        let results = Arc::new(Mutex::new(Vec::new()));

                        // Spawn worker threads
                        let mut handles = vec![];
                        for _ in 0..thread_count {
                            let queue = Arc::clone(&work_queue);
                            let results = Arc::clone(&results);

                            let handle = thread::spawn(move || loop {
                                let work_item = {
                                    let mut queue = queue.lock().unwrap();
                                    queue.pop()
                                };

                                let (idx, chunk) = match work_item {
                                    Some(item) => item,
                                    None => break,
                                };

                                let compressed = implode_bytes(
                                    &chunk,
                                    CompressionMode::Binary,
                                    DictionarySize::Size4K,
                                )
                                .expect("Compression failed");

                                let mut results = results.lock().unwrap();
                                results.push((idx, compressed));
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.join().unwrap();
                        }

                        // Return sorted compressed chunks
                        let mut results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
                        results.sort_by_key(|(idx, _)| *idx);
                        results
                    });
                });
            }
        }
    }

    group.finish();
}

/// Benchmark parallel decompression
fn parallel_decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_decompression");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(20);

    // Generate compressed files for decompression
    let test_cases = vec![
        (10, 102400), // 10 files × 100KB each
        (100, 10240), // 100 files × 10KB each
    ];

    for (file_count, original_size) in test_cases {
        // Generate and compress files
        let compressed_files: Vec<Vec<u8>> = generate_test_files(file_count, original_size)
            .into_iter()
            .map(|data| {
                implode_bytes(&data, CompressionMode::Binary, DictionarySize::Size4K)
                    .expect("Compression failed")
            })
            .collect();

        let total_original_size = file_count * original_size;

        for thread_count in [1, 2, 4, 8].iter() {
            let benchmark_id = BenchmarkId::from_parameter(format!(
                "{}files_{}KB_{}threads",
                file_count,
                original_size / 1024,
                thread_count
            ));

            group.throughput(Throughput::Bytes(total_original_size as u64));
            group.bench_with_input(
                benchmark_id,
                &compressed_files,
                |b, files: &Vec<Vec<u8>>| {
                    b.iter(|| {
                        let files = files.clone();
                        let thread_count = *thread_count;

                        let work_queue = Arc::new(Mutex::new(
                            files.into_iter().enumerate().collect::<Vec<_>>(),
                        ));
                        let results = Arc::new(Mutex::new(Vec::new()));

                        let mut handles = vec![];
                        for _ in 0..thread_count {
                            let queue = Arc::clone(&work_queue);
                            let results = Arc::clone(&results);

                            let handle = thread::spawn(move || loop {
                                let work_item = {
                                    let mut queue = queue.lock().unwrap();
                                    queue.pop()
                                };

                                let (idx, data) = match work_item {
                                    Some(item) => item,
                                    None => break,
                                };

                                let decompressed =
                                    explode_bytes(&data).expect("Decompression failed");

                                let mut results = results.lock().unwrap();
                                results.push((idx, decompressed));
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.join().unwrap();
                        }

                        let mut results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
                        results.sort_by_key(|(idx, _)| *idx);
                        results
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark thread scaling efficiency
fn thread_scaling_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_scaling");
    group.measurement_time(Duration::from_secs(10));

    // Fixed workload: 100 files of 10KB each
    let files = generate_test_files(100, 10240);
    let total_size = 100 * 10240;

    // Test scaling from 1 to 16 threads
    for thread_count in [1, 2, 3, 4, 6, 8, 12, 16].iter() {
        let benchmark_id = BenchmarkId::from_parameter(format!("{thread_count}threads"));

        group.throughput(Throughput::Bytes(total_size as u64));
        group.bench_with_input(benchmark_id, &files, |b, files: &Vec<Vec<u8>>| {
            b.iter(|| {
                let files = files.clone();
                let thread_count = *thread_count;

                let work_queue = Arc::new(Mutex::new(files.into_iter().collect::<Vec<_>>()));
                let results = Arc::new(Mutex::new(0usize));

                let mut handles = vec![];
                for _ in 0..thread_count {
                    let queue = Arc::clone(&work_queue);
                    let results = Arc::clone(&results);

                    let handle = thread::spawn(move || {
                        let mut local_processed = 0;

                        loop {
                            let work_item = {
                                let mut queue = queue.lock().unwrap();
                                queue.pop()
                            };

                            let data = match work_item {
                                Some(item) => item,
                                None => break,
                            };

                            let compressed = implode_bytes(
                                &data,
                                CompressionMode::Binary,
                                DictionarySize::Size4K,
                            )
                            .expect("Compression failed");

                            black_box(compressed);
                            local_processed += 1;
                        }

                        let mut results = results.lock().unwrap();
                        *results += local_processed;
                    });

                    handles.push(handle);
                }

                for handle in handles {
                    handle.join().unwrap();
                }

                let processed = *results.lock().unwrap();
                assert_eq!(processed, 100);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    parallel_file_compression,
    parallel_chunk_compression,
    parallel_decompression,
    thread_scaling_efficiency
);
criterion_main!(benches);
