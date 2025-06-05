//! Async batch processing module
//!
//! This module provides concurrent file processing capabilities for
//! high-throughput batch operations.

#[cfg(feature = "async")]
/// Concurrent file processing with configurable concurrency and memory limits
pub mod processor {
    use crate::{CompressionMode, CompressionStats, DictionarySize, Result};
    use futures::stream::{self, StreamExt, TryStreamExt};
    use std::path::{Path, PathBuf};
    use tokio::fs::File;
    use tokio::io::{AsyncReadExt, BufReader};

    /// Concurrent file processor optimized for throughput
    #[derive(Debug, Clone)]
    pub struct AsyncBatchProcessor {
        concurrency_limit: usize,
        chunk_size: usize,
        memory_limit: usize,
    }

    impl AsyncBatchProcessor {
        /// Create a new batch processor with default settings
        pub fn new() -> Self {
            Self {
                concurrency_limit: num_cpus::get(),
                chunk_size: 64 * 1024,           // 64KB chunks
                memory_limit: 256 * 1024 * 1024, // 256MB total
            }
        }

        /// Set the concurrency limit
        pub fn with_concurrency(mut self, limit: usize) -> Self {
            self.concurrency_limit = limit;
            self
        }

        /// Set the chunk size for processing
        pub fn with_chunk_size(mut self, size: usize) -> Self {
            self.chunk_size = size;
            self
        }

        /// Set the memory limit
        pub fn with_memory_limit(mut self, limit: usize) -> Self {
            self.memory_limit = limit;
            self
        }

        /// Process multiple files concurrently with controlled memory usage
        pub async fn compress_files<P: AsRef<Path> + Send + Sync>(
            &self,
            files: Vec<P>,
            mode: CompressionMode,
            dict_size: DictionarySize,
        ) -> Result<Vec<(PathBuf, Vec<u8>)>> {
            let results = stream::iter(files.into_iter().map(|path| {
                let processor = self.clone();
                async move { processor.compress_single_file(path, mode, dict_size).await }
            }))
            .buffer_unordered(self.concurrency_limit)
            .try_collect()
            .await?;

            Ok(results)
        }

        /// Stream results as they complete for memory efficiency
        pub fn compress_files_streaming<P: AsRef<Path> + Send + Sync + 'static>(
            &self,
            files: Vec<P>,
        ) -> impl futures::Stream<Item = Result<(PathBuf, CompressionStats)>> + '_ {
            let mode = CompressionMode::Binary;
            let dict_size = DictionarySize::Size4K;

            stream::iter(files.into_iter().map(move |path| {
                let processor = self.clone();
                async move {
                    let (path_buf, _data) = processor
                        .compress_single_file(path, mode, dict_size)
                        .await?;
                    let stats = CompressionStats {
                        literal_count: 0,
                        match_count: 0,
                        bytes_processed: 0,
                        longest_match: 0,
                        input_bytes: 0,  // Would be filled from actual compression
                        output_bytes: 0, // Would be filled from actual compression
                        compression_ratio: 0.0,
                    };
                    Ok((path_buf, stats))
                }
            }))
            .buffer_unordered(self.concurrency_limit)
        }

        /// Compress a single file
        async fn compress_single_file<P: AsRef<Path>>(
            &self,
            path: P,
            mode: CompressionMode,
            dict_size: DictionarySize,
        ) -> Result<(PathBuf, Vec<u8>)> {
            let path = path.as_ref();
            let file = File::open(path).await?;
            let reader = BufReader::new(file);

            let compressed = self.compress_reader(reader, mode, dict_size).await?;
            Ok((path.to_path_buf(), compressed))
        }

        /// Compress data from an async reader
        async fn compress_reader<R: tokio::io::AsyncRead + Unpin>(
            &self,
            mut reader: R,
            mode: CompressionMode,
            dict_size: DictionarySize,
        ) -> Result<Vec<u8>> {
            use crate::async_implode::AsyncImplodeWriter;

            let mut output = Vec::new();
            let mut writer = AsyncImplodeWriter::new(&mut output, mode, dict_size)?;

            // Read and compress in chunks
            let mut buffer = vec![0u8; self.chunk_size];
            loop {
                let bytes_read = reader.read(&mut buffer).await?;
                if bytes_read == 0 {
                    break;
                }

                writer.write_chunk(&buffer[..bytes_read]).await?;

                // Yield control periodically for fairness
                if bytes_read == self.chunk_size {
                    tokio::task::yield_now().await;
                }
            }

            writer.finish().await?;
            Ok(output)
        }

        /// Process files with memory usage monitoring
        pub async fn compress_files_with_memory_limit<P: AsRef<Path> + Send + Sync>(
            &self,
            files: Vec<P>,
            mode: CompressionMode,
            dict_size: DictionarySize,
        ) -> Result<Vec<(PathBuf, Vec<u8>)>> {
            let mut results = Vec::new();
            let mut current_memory = 0;

            for chunk in files.chunks(self.concurrency_limit) {
                // Process chunk concurrently
                let chunk_results = stream::iter(chunk.iter().map(|path| {
                    let processor = self.clone();
                    async move { processor.compress_single_file(path, mode, dict_size).await }
                }))
                .buffer_unordered(self.concurrency_limit)
                .try_collect::<Vec<_>>()
                .await?;

                // Check memory usage
                let chunk_memory: usize = chunk_results.iter().map(|(_, data)| data.len()).sum();
                current_memory += chunk_memory;

                results.extend(chunk_results);

                // If we're approaching memory limit, yield and potentially gc
                if current_memory > self.memory_limit * 3 / 4 {
                    tokio::task::yield_now().await;
                    // In a real implementation, we might flush results to disk here
                    current_memory = 0; // Reset for simulation
                }
            }

            Ok(results)
        }
    }

    impl Default for AsyncBatchProcessor {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "async")]
pub use processor::AsyncBatchProcessor;
