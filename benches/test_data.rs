/// Test data generation utilities for benchmarks
use std::fs;
use std::path::Path;

pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate Calgary Corpus-like test files
    pub fn generate_calgary_like_files(output_dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(output_dir)?;

        // Book text (similar to book1 from Calgary Corpus)
        let book_text = Self::generate_book_text(768771);
        fs::write(output_dir.join("book.txt"), book_text)?;

        // News articles (similar to news from Calgary Corpus)
        let news_text = Self::generate_news_text(377109);
        fs::write(output_dir.join("news.txt"), news_text)?;

        // Source code (similar to progc from Calgary Corpus)
        let source_code = Self::generate_source_code(39611);
        fs::write(output_dir.join("source.c"), source_code)?;

        // Binary executable (similar to obj1 from Calgary Corpus)
        let binary_data = Self::generate_binary_executable(21504);
        fs::write(output_dir.join("binary.bin"), binary_data)?;

        // Structured data (similar to trans from Calgary Corpus)
        let structured = Self::generate_structured_data(93695);
        fs::write(output_dir.join("structured.dat"), structured)?;

        Ok(())
    }

    /// Generate Silesia Corpus-like test files
    pub fn generate_silesia_like_files(output_dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(output_dir)?;

        // XML data
        let xml_data = Self::generate_xml_data(5345280);
        fs::write(output_dir.join("data.xml"), xml_data)?;

        // Database dump
        let db_dump = Self::generate_database_dump(10085684);
        fs::write(output_dir.join("database.sql"), db_dump)?;

        // Log file
        let log_data = Self::generate_log_file(20971520);
        fs::write(output_dir.join("server.log"), log_data)?;

        Ok(())
    }

    fn generate_book_text(size: usize) -> Vec<u8> {
        let chapters = [
            "It was the best of times, it was the worst of times, it was the age of wisdom, ",
            "it was the age of foolishness, it was the epoch of belief, it was the epoch of incredulity, ",
            "it was the season of Light, it was the season of Darkness, it was the spring of hope, ",
            "it was the winter of despair, we had everything before us, we had nothing before us, ",
            "we were all going direct to Heaven, we were all going direct the other way. ",
        ];

        let mut result = Vec::with_capacity(size);
        let mut chapter_idx = 0;

        while result.len() < size {
            result.extend_from_slice(chapters[chapter_idx % chapters.len()].as_bytes());
            result.extend_from_slice(b"\n\n");
            chapter_idx += 1;

            // Add some variation
            if chapter_idx % 10 == 0 {
                result.extend_from_slice(b"Chapter ");
                result.extend_from_slice(chapter_idx.to_string().as_bytes());
                result.extend_from_slice(b"\n==========\n\n");
            }
        }

        result.truncate(size);
        result
    }

    fn generate_news_text(size: usize) -> Vec<u8> {
        let headlines = [
            "Breaking News: Technology Advances Continue to Shape Our Future\n",
            "Local Community Celebrates Annual Festival with Record Attendance\n",
            "Scientists Discover New Species in Remote Rainforest Region\n",
            "Economic Report Shows Mixed Results for Third Quarter Growth\n",
            "Weather Update: Sunny Skies Expected Throughout the Weekend\n",
        ];

        let articles = [
            "In a groundbreaking development today, researchers announced ",
            "According to sources close to the matter, officials stated ",
            "The latest reports indicate significant progress in the field ",
            "Community leaders gathered yesterday to discuss important ",
            "Experts predict continued growth in the coming months as ",
        ];

        let mut result = Vec::with_capacity(size);
        let mut idx = 0;

        while result.len() < size {
            result.extend_from_slice(headlines[idx % headlines.len()].as_bytes());
            result.extend_from_slice(b"---\n");
            result.extend_from_slice(articles[idx % articles.len()].as_bytes());
            result.extend_from_slice(b"the situation continues to develop.\n\n");
            idx += 1;
        }

        result.truncate(size);
        result
    }

    fn generate_source_code(size: usize) -> Vec<u8> {
        let code_snippets = vec![
            "#include <stdio.h>\n#include <stdlib.h>\n\n",
            "int process_data(const char* input, size_t length) {\n",
            "    if (input == NULL || length == 0) {\n",
            "        return -1;\n    }\n",
            "    for (size_t i = 0; i < length; i++) {\n",
            "        if (input[i] < 0x20 || input[i] > 0x7E) {\n",
            "            printf(\"Invalid character at position %zu\\n\", i);\n",
            "        }\n    }\n    return 0;\n}\n\n",
            "typedef struct {\n    int id;\n    char name[64];\n    double value;\n} DataRecord;\n\n",
            "void sort_records(DataRecord* records, size_t count) {\n",
            "    // Simple bubble sort for demonstration\n",
            "    for (size_t i = 0; i < count - 1; i++) {\n",
            "        for (size_t j = 0; j < count - i - 1; j++) {\n",
            "            if (records[j].value > records[j + 1].value) {\n",
            "                DataRecord temp = records[j];\n",
            "                records[j] = records[j + 1];\n",
            "                records[j + 1] = temp;\n",
            "            }\n        }\n    }\n}\n\n",
        ];

        let mut result = Vec::with_capacity(size);
        let mut idx = 0;

        while result.len() < size {
            result.extend_from_slice(code_snippets[idx % code_snippets.len()].as_bytes());
            idx += 1;
        }

        result.truncate(size);
        result
    }

    fn generate_binary_executable(size: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(size);

        // ELF header-like structure
        result.extend_from_slice(&[0x7F, b'E', b'L', b'F', 2, 1, 1, 0]);
        result.extend_from_slice(&[0; 8]); // padding
        result.extend_from_slice(&[2, 0, 0x3E, 0]); // type and machine

        // Generate pseudo-binary content
        let mut seed = 0x12345678u32;
        while result.len() < size {
            // Mix of patterns found in real executables
            match (result.len() / 256) % 4 {
                0 => {
                    // Code-like section
                    for _ in 0..16 {
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        result.push((seed % 256) as u8);
                    }
                }
                1 => {
                    // String table section
                    let strings = [
                        &b"main\0"[..],
                        &b"printf\0"[..],
                        &b"malloc\0"[..],
                        &b"free\0"[..],
                    ];
                    result.extend_from_slice(strings[seed as usize % strings.len()]);
                }
                2 => {
                    // Data section with some patterns
                    result.extend_from_slice(&(seed as i32).to_le_bytes());
                    result.extend_from_slice(&[0x00, 0xFF, 0x00, 0xFF]);
                }
                _ => {
                    // Padding/alignment
                    result.extend_from_slice(&[0x90; 16]); // NOP sled
                }
            }
        }

        result.truncate(size);
        result
    }

    fn generate_structured_data(size: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(size);
        let mut record_id = 1000;

        while result.len() < size {
            // Generate CSV-like records
            let item_name = format!("ITEM_{:04}", record_id % 1000);
            let record = format!(
                "{},{},{},{:.2},{}\n",
                record_id,
                item_name,
                (record_id * 7) % 100,              // quantity
                (record_id as f64 * 1.23) % 1000.0, // price
                if record_id % 2 == 0 {
                    "ACTIVE"
                } else {
                    "INACTIVE"
                }
            );

            result.extend_from_slice(record.as_bytes());
            record_id += 1;
        }

        result.truncate(size);
        result
    }

    fn generate_xml_data(size: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(size);

        result.extend_from_slice(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        result.extend_from_slice(b"<records>\n");

        let mut id = 1;
        while result.len() < size - 100 {
            // Leave room for closing tag
            let entry = format!(
                "  <record id=\"{}\">\n    <title>Record {}</title>\n    <value>{}</value>\n    <status>{}</status>\n  </record>\n",
                id,
                id,
                (id * 13) % 1000,
                if id % 3 == 0 { "pending" } else { "complete" }
            );

            result.extend_from_slice(entry.as_bytes());
            id += 1;
        }

        result.extend_from_slice(b"</records>\n");
        result.truncate(size);
        result
    }

    fn generate_database_dump(size: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(size);

        // SQL header
        result.extend_from_slice(b"-- Database dump\n");
        result.extend_from_slice(b"SET SQL_MODE = \"NO_AUTO_VALUE_ON_ZERO\";\n");
        result.extend_from_slice(b"START TRANSACTION;\n\n");

        // Table structure
        result.extend_from_slice(b"CREATE TABLE IF NOT EXISTS `users` (\n");
        result.extend_from_slice(b"  `id` int(11) NOT NULL AUTO_INCREMENT,\n");
        result.extend_from_slice(b"  `username` varchar(50) NOT NULL,\n");
        result.extend_from_slice(b"  `email` varchar(100) NOT NULL,\n");
        result.extend_from_slice(b"  `created_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,\n");
        result.extend_from_slice(b"  PRIMARY KEY (`id`)\n");
        result.extend_from_slice(b") ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;\n\n");

        // Insert statements
        let mut user_id = 1;
        while result.len() < size - 100 {
            let insert = format!(
                "INSERT INTO `users` (`id`, `username`, `email`) VALUES ({user_id}, 'user{user_id}', 'user{user_id}@example.com');\n"
            );

            result.extend_from_slice(insert.as_bytes());
            user_id += 1;
        }

        result.extend_from_slice(b"\nCOMMIT;\n");
        result.truncate(size);
        result
    }

    fn generate_log_file(size: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(size);
        let log_levels = ["INFO", "DEBUG", "WARN", "ERROR"];
        let modules = ["auth", "database", "api", "cache", "queue"];
        let messages = [
            "Request processed successfully",
            "Connection established",
            "Query executed in 23ms",
            "Cache miss, fetching from database",
            "User authentication completed",
            "Background job started",
            "Configuration reloaded",
            "Memory usage: 67%",
        ];

        let mut timestamp = 1640995200; // 2022-01-01 00:00:00
        let mut idx = 0;

        while result.len() < size {
            let log_entry = format!(
                "[{}] {} [{}] {}\n",
                timestamp,
                log_levels[idx % log_levels.len()],
                modules[idx % modules.len()],
                messages[idx % messages.len()]
            );

            result.extend_from_slice(log_entry.as_bytes());
            timestamp += (idx % 60) + 1; // Vary time intervals
            idx += 1;
        }

        result.truncate(size);
        result
    }
}
