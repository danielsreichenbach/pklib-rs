# Changelog

All notable changes to PKLib will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Future Enhancements

- Performance benchmarks with `criterion`
- WASM compatibility
- Async API support

## [0.2.0] - 2025-07-05

### Fixed

- Corrected documentation to accurately describe PKWare DCL format (1980s DOS era) instead of ZIP IMPLODE/EXPLODE format (1990s)
- Updated all documentation references to reflect that this implements PKWare Data Compression Library format with Patent No. 5,051,745
- Clarified that the implementation uses Huffman coding and sliding dictionary compression for game archives like MPQ

## [0.1.0] - 2025-06-05

### Added - Complete Implementation

- **Core Infrastructure**: Complete type system, error handling, and PKLib lookup tables
- **Decompression (Explode)**: Full PKLib-compatible decompression with streaming `ExplodeReader`
- **Compression (Implode)**: Complete PKLib-compatible compression with streaming `ImplodeWriter`
- **All Compression Modes**: Binary and ASCII modes with 1KB/2KB/4KB dictionary support
- **Extended Features**: Maximum repetition length support (516 bytes) and 4-tier length encoding
- **APIs**: Both convenience functions (`explode_bytes`, `implode_bytes`) and streaming APIs
- **CLI Tool**: Full-featured command-line interface (`blast-cli`) with compress/decompress/info commands
- **Testing**: Comprehensive test suite including PKLib compatibility tests and property-based testing
- **Quality**: Production-ready codebase with zero warnings and comprehensive documentation

### Features

- ✅ **100% PKLib Compatibility**: Verified bit-for-bit compatibility with original PKLib
- ✅ **Memory Safety**: Written in safe Rust with comprehensive error handling
- ✅ **Performance**: Efficient algorithms competitive with original C implementation
- ✅ **Streaming Support**: Standard `Read`/`Write` trait implementations
- ✅ **Round-trip Validation**: Complete compression/decompression cycle testing

---

## Development Phases

### Phase 1: Core Infrastructure ✅ Complete

- [x] Static lookup tables module (`tables.rs`)
- [x] PKLib-compatible CRC32 implementation (`crc32.rs`)
- [x] Core types and error handling (`common.rs`, `error.rs`)
- [x] Library entry point and public API (`lib.rs`)
- [x] Comprehensive documentation and tests

### Phase 2: Decompression (Explode) ✅ Complete

- [x] Bit reader implementation with PKLib compatibility
- [x] Huffman decoding functions for all modes
- [x] Sliding window decompression with extended length support
- [x] `ExplodeReader` with `Read` trait implementation
- [x] Full compatibility tests with PKLib reference data

### Phase 3: Compression (Implode) ✅ Complete

- [x] Hash table implementation for pattern matching
- [x] Pattern matching with distance validation
- [x] Bit writer implementation with proper alignment
- [x] `ImplodeWriter` with `Write` trait implementation
- [x] Round-trip compatibility verification

### Phase 4: Testing and Validation ✅ Complete

- [x] Comprehensive test suite with PKLib test data
- [x] Property-based testing with `proptest`
- [x] Systematic debugging and compatibility verification
- [x] Code quality cleanup and documentation
- [x] Production-ready implementation with all edge cases handled

### Future Enhancements 💭 Ideas

- [ ] Command-line interface tool
- [ ] WASM compatibility
- [ ] Async API support
- [ ] SIMD optimizations
- [ ] Thread-safe concurrent processing
