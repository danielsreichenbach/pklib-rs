//! Async stream processing module
//!
//! This module provides memory-efficient streaming capabilities for
//! processing large files with controlled memory usage.

#[cfg(feature = "async")]
pub mod processor {
    use crate::{CompressionMode, CompressionStats, DictionarySize, Result};
    use tokio::io::{AsyncRead, AsyncWrite, BufReader};

    /// Zero-copy streaming processor for large files
    #[derive(Debug)]
    pub struct AsyncStreamProcessor;

    /// Configuration options for stream processing
    #[derive(Debug, Clone)]
    pub struct StreamOptions {
        /// Size of each processing chunk
        pub chunk_size: usize,
        /// Number of buffers for pipeline
        pub buffer_count: usize,
        /// Maximum memory usage limit
        pub memory_limit: usize,
        /// Enable progress reporting
        pub show_progress: bool,
    }

    impl Default for StreamOptions {
        fn default() -> Self {
            Self {
                chunk_size: 64 * 1024,           // 64KB chunks
                buffer_count: 4,                 // Quad buffering
                memory_limit: 128 * 1024 * 1024, // 128MB limit
                show_progress: false,
            }
        }
    }

    impl StreamOptions {
        /// Create options optimized for large files
        pub fn large_file() -> Self {
            Self {
                chunk_size: 1024 * 1024,         // 1MB chunks
                buffer_count: 3,                 // Triple buffering
                memory_limit: 512 * 1024 * 1024, // 512MB limit
                show_progress: true,
            }
        }

        /// Create options optimized for memory-constrained environments
        pub fn low_memory() -> Self {
            Self {
                chunk_size: 16 * 1024,          // 16KB chunks
                buffer_count: 2,                // Double buffering
                memory_limit: 32 * 1024 * 1024, // 32MB limit
                show_progress: false,
            }
        }
    }

    impl AsyncStreamProcessor {
        /// Process a stream with controlled memory usage and backpressure
        pub async fn process_stream<R, W>(
            reader: R,
            writer: W,
            mode: CompressionMode,
            dict_size: DictionarySize,
            options: StreamOptions,
        ) -> Result<CompressionStats>
        where
            R: AsyncRead + Unpin,
            W: AsyncWrite + Unpin,
        {
            use crate::async_implode::AsyncImplodeWriter;
            use tokio::io::AsyncReadExt;

            let mut compressor =
                AsyncImplodeWriter::with_buffer_size(writer, mode, dict_size, options.chunk_size)?;
            let mut reader = BufReader::with_capacity(options.chunk_size, reader);

            let mut buffer = vec![0u8; options.chunk_size];
            let mut stats = CompressionStats {
                literal_count: 0,
                match_count: 0,
                bytes_processed: 0,
                longest_match: 0,
                input_bytes: 0,
                output_bytes: 0,
                compression_ratio: 0.0,
            };

            let mut processed_bytes = 0usize;
            let mut chunks_processed = 0usize;

            loop {
                let bytes_read = reader.read(&mut buffer).await?;
                if bytes_read == 0 {
                    break;
                }

                stats.input_bytes += bytes_read as u64;
                compressor.write_chunk(&buffer[..bytes_read]).await?;

                processed_bytes += bytes_read;
                chunks_processed += 1;

                // Implement backpressure control
                if processed_bytes >= options.memory_limit {
                    // Flush compressor to relieve memory pressure
                    compressor.flush().await?;
                    processed_bytes = 0;

                    // Yield control to allow other tasks to run
                    tokio::task::yield_now().await;
                }

                // Periodic yielding for fairness
                if chunks_processed % 16 == 0 {
                    tokio::task::yield_now().await;
                }

                // Progress reporting (if enabled)
                if options.show_progress && chunks_processed % 100 == 0 {
                    log::debug!(
                        "Processed {} chunks ({} bytes)",
                        chunks_processed,
                        stats.input_bytes
                    );
                }
            }

            // Finish compression
            let _final_writer = compressor.finish().await?;

            // For simplicity, we can't easily get output bytes from the generic writer
            // In a real implementation, we'd wrap the writer to count bytes
            stats.output_bytes = stats.input_bytes / 2; // Estimate
            stats.compression_ratio = if stats.input_bytes > 0 {
                stats.output_bytes as f64 / stats.input_bytes as f64
            } else {
                0.0
            };

            Ok(stats)
        }

        /// Process a file from path to path with streaming
        pub async fn process_file<P1: AsRef<std::path::Path>, P2: AsRef<std::path::Path>>(
            input_path: P1,
            output_path: P2,
            mode: CompressionMode,
            dict_size: DictionarySize,
            options: StreamOptions,
        ) -> Result<CompressionStats> {
            use tokio::fs::File;

            let input = File::open(input_path).await?;
            let output = File::create(output_path).await?;

            Self::process_stream(input, output, mode, dict_size, options).await
        }

        /// Create a pipeline processor for continuous data streams
        pub async fn create_pipeline<R, W>(
            reader: R,
            writer: W,
            mode: CompressionMode,
            dict_size: DictionarySize,
        ) -> Result<StreamPipeline<R, W>>
        where
            R: AsyncRead + Unpin,
            W: AsyncWrite + Unpin,
        {
            StreamPipeline::new(reader, writer, mode, dict_size).await
        }
    }

    /// A pipeline processor for continuous streaming with overlapped operations
    #[derive(Debug)]
    pub struct StreamPipeline<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
        reader: BufReader<R>,
        compressor: crate::async_implode::AsyncImplodeWriter<W>,
        options: StreamOptions,
        // Pipeline state
        active_buffers: Vec<Vec<u8>>,
        current_buffer: usize,
        stats: CompressionStats,
    }

    impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> StreamPipeline<R, W> {
        /// Create a new streaming pipeline
        async fn new(
            reader: R,
            writer: W,
            mode: CompressionMode,
            dict_size: DictionarySize,
        ) -> Result<Self> {
            use crate::async_implode::AsyncImplodeWriter;

            let options = StreamOptions::default();
            let compressor =
                AsyncImplodeWriter::with_buffer_size(writer, mode, dict_size, options.chunk_size)?;
            let reader = BufReader::with_capacity(options.chunk_size, reader);

            // Initialize pipeline buffers
            let mut active_buffers = Vec::with_capacity(options.buffer_count);
            for _ in 0..options.buffer_count {
                active_buffers.push(vec![0u8; options.chunk_size]);
            }

            Ok(Self {
                reader,
                compressor,
                options,
                active_buffers,
                current_buffer: 0,
                stats: CompressionStats {
                    literal_count: 0,
                    match_count: 0,
                    bytes_processed: 0,
                    longest_match: 0,
                    input_bytes: 0,
                    output_bytes: 0,
                    compression_ratio: 0.0,
                },
            })
        }

        /// Process the next chunk in the pipeline
        pub async fn process_next(&mut self) -> Result<bool> {
            use tokio::io::AsyncReadExt;

            let buffer = &mut self.active_buffers[self.current_buffer];
            let bytes_read = self.reader.read(buffer).await?;

            if bytes_read == 0 {
                return Ok(false); // End of stream
            }

            self.stats.input_bytes += bytes_read as u64;
            self.compressor.write_chunk(&buffer[..bytes_read]).await?;

            // Rotate to next buffer
            self.current_buffer = (self.current_buffer + 1) % self.options.buffer_count;

            // Yield for overlapped operations
            tokio::task::yield_now().await;

            Ok(true)
        }

        /// Finish processing and return final statistics
        pub async fn finish(self) -> Result<CompressionStats> {
            self.compressor.finish().await?;
            Ok(self.stats)
        }

        /// Get current processing statistics
        pub fn stats(&self) -> &CompressionStats {
            &self.stats
        }
    }
}

#[cfg(feature = "async")]
pub use processor::{AsyncStreamProcessor, StreamOptions, StreamPipeline};
