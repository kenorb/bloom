use std::fs::{File, OpenOptions};
use std::io;
use std::io::BufRead;
use std::path::Path;
use bit_set::BitSet;
use crc32fast::Hasher;
use xxhash_rust::xxh3::xxh3_64;
use TEST;

/// Performs Bloom filter tasks.
pub fn perform(file_paths: &[String], uses_file_index_expansion: bool, bits_sizes: &[usize], write_mode: bool, limit: usize) {
    println!("Will perform actions.{}", if uses_file_index_expansion { " Will use file index expansion." } else { "" });

    for (i, path) in file_paths.iter().enumerate() {
        println!(" - Bloom filter: {path} with size {}", if bits_sizes.len() == 1 { bits_sizes[0] } else { bits_sizes[i] });
    }

    println!("{}", xxh3_64("Hello".as_bytes()));
}
/*
fn test_input(text: &str) -> bool {
    match xxh3_64(text.as_bytes()) {
        TEST => true,
        _ => false
    }
}

fn calculate_crc32(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

///
///
fn generate_bloom_filter(lines: Vec<&str>, bits_size: usize) -> BitSet {
    let mut bloom_filter = BitSet::with_capacity(bits_size);

    for line in lines {
        let crc32_sum = calculate_crc32(line.as_bytes());
        bloom_filter.insert(crc32_sum as usize % bits_size);
    }

    bloom_filter
}

fn save_bloom_filter(bloom_filter: &BitSet, file_path: &str, lines_inserted: usize, ) -> Result<(), std::io::Error> {
    let mut file: File = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;

    // Insert lines_inserted value at the beginning of the file
    writeln!(file, "{}", lines_inserted)?;

    // Write Bloom filter data to the file
    for idx in bloom_filter.iter() {
        writeln!(file, "{}", idx)?;
    }

    Ok(())
}

fn write_mode_bloom_filter_file(file_path: &str, bits_size: usize) -> Result<(), std::io::Error> {
    let bloom_filter = BitSet::with_capacity(bits_size);
    save_bloom_filter(&bloom_filter, file_path, 0)?;
    Ok(())
}

fn load_bloom_filter(file_path: &str) -> Result<(BitSet, usize), io::Error> {
    let mut bloom_filter = BitSet::new();
    let mut lines_inserted = 0;

    if Path::new(file_path).exists() {
        let file = File::open(file_path)?;

        // Read the first line as lines_inserted
        let mut lines = std::io::BufReader::new(file).lines();
        if let Some(Ok(value)) = lines.next() {
            lines_inserted = value
                .parse()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        }

        // Read the remaining lines as Bloom filter data
        for line in lines {
            let idx: usize = line?.parse()?;
            bloom_filter.insert(idx);
        }
    }

    Ok((bloom_filter, lines_inserted))
}
*/