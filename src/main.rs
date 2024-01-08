extern crate bit_set;
extern crate crc32fast;

use bit_set::BitSet;
use crc32fast::Hasher;
use io::{BufReader};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{stdin, stdout, BufRead, Write};
use std::path::Path;

fn calculate_crc32(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

fn generate_bloom_filter(lines: Vec<&str>, bits_size: usize) -> BitSet {
    let mut bloom_filter = BitSet::with_capacity(bits_size);

    for line in lines {
        let crc32_sum = calculate_crc32(line.as_bytes());
        bloom_filter.insert(crc32_sum as usize % bits_size);
    }

    bloom_filter
}

fn save_bloom_filter(bloom_filter: &BitSet, file_path: &str, lines_inserted: usize) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(file_path)?;

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
        let mut lines = io::BufReader::new(file).lines();
        if let Some(Ok(value)) = lines.next() {
            lines_inserted = value.parse().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, e)
            })?;
        }

        // Read the remaining lines as Bloom filter data
        for line in lines {
            let idx: usize = line?.parse()?;
            bloom_filter.insert(idx);
        }
    }

    Ok((bloom_filter, lines_inserted))
}

fn print_help() {
    println!("Bloom Filter Command Line Utility");
    println!("Usage: bloom_filter [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -f, --file FILE   Specify Bloom filter file");
    println!("  -b, --bits BITS   Specify bits size for the Bloom filter (default: 1,000,000)");
    println!("  -w, --write       Create an empty Bloom filter file or update an existing one");
    println!("  -l, --limit LIMIT Limit the number of lines to insert into the Bloom filter");
    println!("  -h, --help        Print help and usage information");
}

fn main() {
    let mut file_paths = Vec::new();
    let mut bits_sizes = Vec::new();
    let mut write_mode = false;
    let mut lines_inserted = 0;
    let mut limit = None;

    // Parse command line arguments
    for (idx, arg) in env::args().enumerate().skip(1) {
        match arg.as_str() {
            "-f" | "--file" => {
                let file_path = env::args().nth(idx + 1).unwrap_or_else(|| {
                    eprintln!("Error: No file path provided after -f or --file parameter.");
                    std::process::exit(1);
                });
                file_paths.push(file_path);
            },
            "-b" | "--bits" => {
                let bits_size = env::args().nth(idx + 1).unwrap_or_else(|| {
                    eprintln!("Error: No bits size provided after -b or --bits parameter.");
                    std::process::exit(1);
                }).parse().unwrap_or_else(|_| {
                    eprintln!("Error: Bits size must be a positive integer.");
                    std::process::exit(1);
                });
                bits_sizes.push(bits_size);
            },
            "-w" | "--write" => write_mode = true,
            "-l" | "--limit" => {
                limit = Some(env::args().nth(idx + 1).unwrap_or_else(|| {
                    eprintln!("Error: No limit value provided after -l or --limit parameter.");
                    std::process::exit(1);
                }).parse().unwrap_or_else(|_| {
                    eprintln!("Error: Limit must be a positive integer.");
                    std::process::exit(1);
                }));
            },
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            },
            _ => (),
        }
    }

    if file_paths.is_empty() {
        eprintln!("Error: No file paths provided.");
        std::process::exit(1);
    }

    if bits_sizes.len() > 1 && bits_sizes.len() != file_paths.len() {
        eprintln!("Error: Number of bits sizes should match the number of file paths.");
        std::process::exit(1);
    }

    if write_mode {
        for (i, file_path) in file_paths.iter().enumerate() {
            let bits_size = if bits_sizes.is_empty() {
                1_000_000 // Default bits size
            } else {
                bits_sizes[i]
            };

            // Create an empty Bloom filter file or update an existing one
            if let Err(err) = write_mode_bloom_filter_file(&file_path, bits_size) {
                eprintln!("Error: {}", err);
                std::process::exit(1);
            }
        }
    }
    else {
        if let Ok((mut bloom_filter, mut current_lines_inserted)) = load_bloom_filter(&file_path) {
            // ...
        } else {
            // Handle the case where loading the Bloom filter fails
            eprintln!("Error: Failed to load Bloom filter from file: {}", file_path);
            std::process::exit(1);
        }
    }

    for line in stdin().lock().lines() {
        let input_line = line.unwrap();
        let crc32_sum = calculate_crc32(input_line.as_bytes());

        // Check if the CRC32 sum is already in the Bloom filter
        if !bloom_filter.contains(crc32_sum as usize % bits_size) {
            if write_mode {
                bloom_filter.insert(crc32_sum as usize % bits_size);
            }
            current_lines_inserted += 1;
        }

        if let Some(limit_value) = limit {
            if current_lines_inserted >= limit_value {
                // If the limit is reached, print the number of lines inserted and exit
                println!("Lines inserted into Bloom filter: {}", current_lines_inserted);
                if write_mode {
                    // @todo: Find first file which is not full and can be written.
                    for (i, file_path) in file_paths.iter().enumerate() {
                        save_bloom_filter(&bloom_filter, &file_path, current_lines_inserted).unwrap_or_else(|err| {
                            eprintln!("Error: Failed to save Bloom filter to file: {}: {}", file_path, err);
                            std::process::exit(1);
                        });
                    }
                }
                return;
            }
        }
    }

    // Print the number of lines inserted into the Bloom filter
    println!("Lines inserted into Bloom filter: {}", lines_inserted);
}
