extern crate bit_set;
extern crate crc32fast;
extern crate xxhash_rust;
extern crate parse_size;
extern crate bloomfilter;
extern crate memory_stats;

mod bloom {
    pub mod containers;
    pub mod process;
}

use std::{env, io};
use std::io::{stdin, stdout, BufRead, BufReader, Write};
use xxhash_rust::const_xxh3::xxh3_64 as const_xxh3;
use parse_size::parse_size;
use bloom::process::process;

const TEST: u64 = const_xxh3(b"TEST");

struct Params {
    debug: bool,
    file_paths: Vec<String>,
    uses_file_index_expansion: bool,
    sizes: Vec<usize>,
    write_mode: bool,
    limit: usize,
    error_rate: f64,
}

fn print_help() {
    println!("Bloom Filter Command Line Utility");
    println!("Usage: bloom_filter [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -f,  --file FILE       Specifies Bloom filter file. You may specify multiple files. You can also specify a single file");
    println!("                         with '#' character that will be automatically expanded to file index.");
    //println!("  -fl, --file-limit NUM   Limits the number of files to be used when path contains '#' file index expansion character.");
    //println!("                          Only applied when writing. For reading all files are used.");
    println!("  -s,  --size NUM[UNIT]  Specifies Bloom filter size in bytes or given unit.");
    println!("  -e,  --error-rate NUM  Specifies number of wanted rate of false positives.");
    println!("  -w,  --write           Creates an empty Bloom filter file or update an existing one.");
    println!("  -l,  --lines NUM       Limits the number of lines to write into the Bloom filter for each file.");
    println!("  -d,  --debug           Will output debug information.");
    println!("  -h,  --help            Prints help and usage information.");
    println!();
    println!("Examples:");
    println!();
    println!("  - Will use and write maximum of two bloom filter files with maximum of 10 lines of input for each file. All other");
    println!("    lines will not be stored in the files:");
    println!("  $ bloom_filter -w -l 10 -f file1.blf -f file2.blf");
    println!();
    println!("  - Will use and write maximum of 20 filter files with maximum of 10 lines of input for each file having 100MiB in size.");
    println!("    In/out file names will be file01.blf - file19:");
    println!("  $ bloom_filter -w -GB 1 -l 10 -f file#.blf");
}

fn main() {
    let mut params = Params {
        debug: false,
        file_paths: vec![],
        error_rate: 0.0,
        uses_file_index_expansion: false,
        sizes: vec![],
        write_mode: false,
        limit: 0,
    };

    // Parse command line arguments.
    for (idx, arg) in env::args().enumerate().skip(1) {
        match arg.as_str() {
            // File output path. Could be passed multiple times.
            "-f" | "--file" => {
                let file_path = env::args().nth(idx + 1).unwrap_or_else(|| {
                    eprintln!("Error: No file path provided after -f or --file parameter.");
                    std::process::exit(1);
                });
                params.file_paths.push(file_path);
            }

            // Size of the Bloom filter file in bits.
            "-b" | "--bytes"  => {
                let mut bits_size: usize = env::args()
                    .nth(idx + 1)
                    .unwrap_or_else(|| {
                        eprintln!("Error: No bits size provided after -b or --bits parameter.");
                        std::process::exit(1);
                    })
                    .parse()
                    .unwrap_or_else(|_| {
                        eprintln!("Error: Bits size must be a positive integer.");
                        std::process::exit(1);
                    });

                params.sizes.push(bits_size);
            }

            // Size of the Bloom filter file in given unit.
            "-s" | "--size" => {
                let mut bits_size:usize = 0;
                let mut bloom_size_str: String = env::args()
                    .nth(idx + 1)
                    .unwrap_or_else(|| {
                        eprintln!("Error: No size provided after -s or --size parameter.");
                        std::process::exit(1);
                    })
                    .parse()
                    .unwrap_or_else(|_| {
                        eprintln!("Error: Size must be a string with optional unit.");
                        std::process::exit(1);
                    });

                bits_size = parse_size(bloom_size_str).unwrap_or_else(|_| {
                    eprintln!("Error: Could not parse Bloom filter size passed via -s or --size parameter.");
                    std::process::exit(1);
                }) as usize;

                params.sizes.push(bits_size);
            }

            // Specifies expected rate of false positives.
            "-e" | "--error-rate" => {
                params.error_rate = env::args()
                    .nth(idx + 1)
                    .unwrap_or_else(|| {
                        eprintln!(
                            "Error: No error rate value provided after -e or --error-rate parameter."
                        );
                        std::process::exit(1);
                    })
                    .parse()
                    .unwrap_or_else(|_| {
                        eprintln!("Error: Error rate must be a positive number.");
                        std::process::exit(1);
                    });
            }

            // Whether we want to update (write to) Bloom filter files.
            "-w" | "--write" => params.write_mode = true,

            // Specifies maximum number of lines that could be added to each Bloom filer file.
            "-l" | "--limit" => {
                params.limit = env::args()
                    .nth(idx + 1)
                    .unwrap_or_else(|| {
                        eprintln!(
                            "Error: No limit value provided after -l or --limit parameter."
                        );
                        std::process::exit(1);
                    })
                    .parse()
                    .unwrap_or_else(|_| {
                        eprintln!("Error: Limit must be a positive integer.");
                        std::process::exit(1);
                    });
            }

            // Will output debug information.
            "-d" | "--debug" => params.debug = true,
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ => (),
        }
    }

    /*
    if params.file_paths.is_empty() {
        eprintln!("Error: No file paths provided.");
        std::process::exit(1);
    }
    */

    if params.sizes.len() > 1 && params.sizes.len() != params.file_paths.len() {
        eprintln!("Error: Number of bits sizes should be exactly one or match the number of file paths.");
        std::process::exit(1);
    }


    // Number of '#' characters in the file path (there could by only one file path with '#' character).
    let mut num_path_hashes: usize = 0;

    for path in &params.file_paths {
        num_path_hashes = path.matches("#").count();
        if num_path_hashes > 1 {
            eprintln!("Error: There can be only one '#' file index expansion character in the file path.");
            std::process::exit(1);
        }
        else if num_path_hashes == 1 {
            if params.file_paths.len() > 1 {
                eprintln!("Error: There can be only one -f or --file path if '#' symbol was used in the file path.");
                std::process::exit(1);
            }
        }
    }

    params.uses_file_index_expansion = num_path_hashes == 1;

    process(&mut params);

    /*

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
    } else {
        if let Ok((mut bloom_filter, mut current_lines_inserted)) = load_bloom_filter(&file_paths) {
            // ...
        } else {
            // Handle the case where loading the Bloom filter fails
            eprintln!(
                "Error: Failed to load Bloom filter from file: {}",
                file_paths
            );
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
                println!(
                    "Lines inserted into Bloom filter: {}",
                    current_lines_inserted
                );
                if write_mode {
                    // @todo: Find first file which is not full and can be written.
                    for (i, file_path) in file_paths.iter().enumerate() {
                        save_bloom_filter(&bloom_filter, &file_path, current_lines_inserted)
                            .unwrap_or_else(|err| {
                                eprintln!(
                                    "Error: Failed to save Bloom filter to file: {}: {}",
                                    file_path, err
                                );
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
  */
}
