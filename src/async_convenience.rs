//! Async convenience functions
//!
//! This module provides easy-to-use async functions for common compression
//! and decompression operations.

#[cfg(feature = "async")]
pub mod functions {
    use crate::{CompressionMode, CompressionStats, DictionarySize, Result};
    use bytes::Bytes;
    use futures::TryStreamExt;
    use std::path::Path;
    use tokio::io::{AsyncRead, AsyncWrite};

    /// Decompress data from an async reader
    pub async fn explode_async<R: AsyncRead + Unpin>(reader: R) -> Result<Vec<u8>> {
        use crate::async_explode::AsyncExplodeReader;

        let mut exploder = AsyncExplodeReader::new(reader)?;
        let mut output = Vec::new();

        while let Some(chunk) = exploder.try_next().await? {
            output.extend_from_slice(&chunk);
        }

        Ok(output)
    }

    /// Compress data from an async reader
    pub async fn implode_async<R: AsyncRead + Unpin>(
        reader: R,
        mode: CompressionMode,
        dict_size: DictionarySize,
    ) -> Result<Vec<u8>> {
        use crate::async_implode::AsyncImplodeWriter;
        use tokio::io::{AsyncReadExt, BufReader};

        let mut output = Vec::new();
        let mut writer = AsyncImplodeWriter::new(&mut output, mode, dict_size)?;
        let mut reader = BufReader::new(reader);

        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            writer.write_chunk(&buffer[..bytes_read]).await?;
        }

        writer.finish().await?;
        Ok(output)
    }

    /// Compress data from bytes
    pub async fn implode_bytes_async(
        data: &[u8],
        mode: CompressionMode,
        dict_size: DictionarySize,
    ) -> Result<Vec<u8>> {
        use std::io::Cursor;
        implode_async(Cursor::new(data), mode, dict_size).await
    }

    /// Decompress data from bytes
    pub async fn explode_bytes_async(data: &[u8]) -> Result<Vec<u8>> {
        use std::io::Cursor;
        explode_async(Cursor::new(data)).await
    }

    /// Compress a file asynchronously
    pub async fn compress_file<P1: AsRef<Path>, P2: AsRef<Path>>(
        input_path: P1,
        output_path: P2,
        mode: CompressionMode,
        dict_size: DictionarySize,
    ) -> Result<CompressionStats> {
        use crate::async_stream::{AsyncStreamProcessor, StreamOptions};

        AsyncStreamProcessor::process_file(
            input_path,
            output_path,
            mode,
            dict_size,
            StreamOptions::default(),
        )
        .await
    }

    /// Decompress a file asynchronously
    pub async fn decompress_file<P1: AsRef<Path>, P2: AsRef<Path>>(
        input_path: P1,
        output_path: P2,
    ) -> Result<CompressionStats> {
        use crate::async_explode::AsyncExplodeReader;
        use tokio::fs::File;
        use tokio::io::{AsyncWriteExt, BufWriter};

        let input = File::open(input_path).await?;
        let output = File::create(output_path).await?;
        let mut writer = BufWriter::new(output);

        let mut reader = AsyncExplodeReader::new(input)?;
        let mut total_bytes = 0u64;

        while let Some(chunk) = reader.try_next().await? {
            writer.write_all(&chunk).await?;
            total_bytes += chunk.len() as u64;
        }

        writer.flush().await?;

        Ok(CompressionStats {
            literal_count: 0,
            match_count: 0,
            bytes_processed: total_bytes as usize,
            longest_match: 0,
            input_bytes: 0, // We'd need to track compressed size
            output_bytes: total_bytes,
            compression_ratio: 0.0,
        })
    }

    /// Compress multiple files concurrently
    pub async fn compress_files<P: AsRef<Path> + Send + Sync>(
        files: Vec<P>,
        mode: CompressionMode,
        dict_size: DictionarySize,
        concurrency: Option<usize>,
    ) -> Result<Vec<(std::path::PathBuf, Vec<u8>)>> {
        use crate::async_batch::AsyncBatchProcessor;

        let mut processor = AsyncBatchProcessor::new();
        if let Some(limit) = concurrency {
            processor = processor.with_concurrency(limit);
        }

        processor.compress_files(files, mode, dict_size).await
    }

    /// Create a streaming compressor for continuous data
    pub async fn create_streaming_compressor<W: AsyncWrite + Unpin>(
        writer: W,
        mode: CompressionMode,
        dict_size: DictionarySize,
    ) -> Result<crate::async_implode::AsyncImplodeWriter<W>> {
        use crate::async_implode::AsyncImplodeWriter;
        AsyncImplodeWriter::new(writer, mode, dict_size)
    }

    /// Create a streaming decompressor for continuous data
    pub async fn create_streaming_decompressor<R: AsyncRead + Unpin>(
        reader: R,
    ) -> Result<crate::async_explode::AsyncExplodeReader<R>> {
        use crate::async_explode::AsyncExplodeReader;
        AsyncExplodeReader::new(reader)
    }

    /// Utilities for working with async streams
    pub mod stream_utils {
        use super::*;
        use futures::Stream;

        /// Convert a vector of data into a stream of chunks
        pub fn data_to_stream(
            data: Vec<u8>,
            chunk_size: usize,
        ) -> impl Stream<Item = Result<Bytes>> {
            futures::stream::iter(
                data.chunks(chunk_size)
                    .map(|chunk| Ok(Bytes::copy_from_slice(chunk)))
                    .collect::<Vec<_>>(),
            )
        }

        /// Collect a stream of chunks back into a vector
        pub async fn stream_to_data<S>(stream: S) -> Result<Vec<u8>>
        where
            S: Stream<Item = Result<Bytes>>,
        {
            let chunks: Vec<Bytes> = stream.try_collect().await?;
            let total_len: usize = chunks.iter().map(|chunk| chunk.len()).sum();
            let mut result = Vec::with_capacity(total_len);

            for chunk in chunks {
                result.extend_from_slice(&chunk);
            }

            Ok(result)
        }

        /// Transform a compression stream with custom processing
        pub fn transform_stream<S, F, Fut>(
            stream: S,
            transform: F,
        ) -> impl Stream<Item = Result<Bytes>>
        where
            S: Stream<Item = Result<Bytes>>,
            F: Fn(Bytes) -> Fut + Send + Sync + Clone + 'static,
            Fut: std::future::Future<Output = Result<Bytes>> + Send,
        {
            use futures::StreamExt;

            stream.then(move |item| {
                let transform = transform.clone();
                async move {
                    match item {
                        Ok(chunk) => transform(chunk).await,
                        Err(e) => Err(e),
                    }
                }
            })
        }
    }
}

#[cfg(feature = "async")]
pub use functions::*;
