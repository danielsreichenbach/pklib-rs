#!/bin/bash
# Example usage of blast-cli tool

echo "=== PKLib CLI Tool Usage Examples ==="
echo

# Create test data
echo "Creating test data..."
echo "Hello, World! This is a test of the PKLib CLI tool." > example.txt
echo "PKLib provides both compression and decompression functionality." >> example.txt
echo "It's compatible with the original PKWare DCL format." >> example.txt

# Show file size
echo "Original file size: $(wc -c < example.txt) bytes"
echo

# Compress with ASCII mode
echo "1. Compressing with ASCII mode..."
cargo run --bin blast-cli -- compress example.txt example.pklib --mode ascii --dict-size size2-k
echo

# Show compressed file info
echo "2. Showing compressed file information..."
cargo run --bin blast-cli -- info example.pklib
echo

# Decompress the file
echo "3. Decompressing..."
cargo run --bin blast-cli -- decompress example.pklib example_restored.txt
echo

# Verify round-trip
echo "4. Verifying round-trip integrity..."
if diff example.txt example_restored.txt > /dev/null; then
    echo "✓ Round-trip successful - files are identical"
else
    echo "✗ Round-trip failed - files differ"
fi
echo

# Test with binary mode
echo "5. Testing binary mode compression..."
cargo run --bin blast-cli -- compress example.txt example_binary.pklib --mode binary --dict-size size4-k
echo

# Compare compression ratios
echo "6. Comparing compression results..."
original_size=$(wc -c < example.txt)
ascii_size=$(wc -c < example.pklib)
binary_size=$(wc -c < example_binary.pklib)

echo "Original:      $original_size bytes"
echo "ASCII mode:    $ascii_size bytes ($(( ascii_size * 100 / original_size ))% ratio)"
echo "Binary mode:   $binary_size bytes ($(( binary_size * 100 / original_size ))% ratio)"
echo

# Cleanup
echo "Cleaning up..."
rm -f example.txt example.pklib example_restored.txt example_binary.pklib
echo "Done!"
