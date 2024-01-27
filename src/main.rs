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

use std::{env};
use std::io::{BufRead};
use xxhash_rust::const_xxh3::xxh3_64 as const_xxh3;
use parse_size::parse_size;
use bloom::process::process;

const TEST: u64 = const_xxh3(b"TEST");

#[derive(Copy, Clone)]
enum DataSource {
    Memory,
    File
}

#[derive(Copy, Clone)]
enum ConstructionType {
    // -ls NUM,NUM[UNIT]
    LinesAndSize,
    // -le NUM,NUM
    LinesAndErrorRate,
}


#[derive(Copy, Clone)]
struct ConstructionDetails {
    construction_type: ConstructionType,
    limit: usize,
    error_rate: f64,
    size: usize,
}

struct ContainerDetails {
    path: String,
    data_source: DataSource,
    construction_details: ConstructionDetails
}

pub struct Params {
    debug: bool,
    containers_details: Vec<ContainerDetails>,
    write_mode: bool,
}

fn print_help() {
    println!("Bloom Filter Command Line Utility");
    println!();
    println!("USAGE:");
    println!("  bloom_filter [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!();
    println!("  -f,   --file FILE                     Specifies Bloom filter file. You may specify multiple files.");
    println!();
    println!("  -w,   --write                         Creates an empty Bloom filter file or updates an existing one.");
    println!();
    println!("  -ls,  --lines-and-size NUM,NUM[UNIT]  First number limits the number of lines to write into the Bloom filter for each");
    println!("                                        file. Second number specifies Bloom filter size in bytes or given unit. Use -ls");
    println!("                                        once to specify settings for all files or use it multiple times for each file.");
    println!("                                        Mutually exclusive with -le option.");
    println!();
    println!("  -le, --lines-and-error-rate NUM,NUM   First number limits the number of lines to write into the Bloom filter for each");
    println!("                                        file. Second number specifies wanted error rate for the given file (> 0 and < 1).");
    println!("                                        Use -le once to specify settings for all files or use it multiple times for each");
    println!("                                        file. Mutually exclusive with -ls option.");
    println!();
    println!("  -l,  --lines NUM                      Limits the number of lines to write into the Bloom filter for each file.");
    println!();
    println!("  -d,  --debug                          Will output debug information.");
    println!();
    println!("  -h,  --help                           Prints help and usage information.");
    println!();
    println!("EXAMPLES:");
    println!();
    println!("  - Will use and write two bloom filter files with maximum of 10 lines and 0.01 error rate each file. All other lines");
    println!("    will not be stored in the files:");
    println!("  $ bloom_filter  -w  -f file1.blf  -f file2.blf  -le 10,0.01  < input.txt");
    println!();
    println!("  - Will use memory and  maximum of 10 lines of input for the filter having 100MiB in size.");
    println!("  $ bloom_filter  -ls 10,100MiB  < input.txt");
}

fn main() {
    let mut params = Params {
        debug: false,
        containers_details: vec![],
        write_mode: false,
    };

    // List of passed file paths.
    let mut file_paths: Vec<String> = vec![];

    // List of passed construction details (pairs of limit and error rate or size).
    let mut constructions_details: Vec<ConstructionDetails> = vec![];

    // Parses file arguments from command line. File construction options will be parsed later and file structs will be
    // filled accordingly.
    for (idx, arg) in env::args().enumerate().skip(1) {
        match arg.as_str() {
            // File output path. Could be passed multiple times.
            "-f" | "--file" => {
                let file_path = env::args().nth(idx + 1).unwrap_or_else(|| {
                    eprintln!("Error: No file path provided after -f or --file parameter.");
                    std::process::exit(1);
                });

                file_paths.push(file_path);
            },

            // Specified limit and size of the Bloom filter file in given unit.
            "-ls" | "--limit-and-size" => {
                let value = env::args().nth(idx + 1).unwrap_or_else(|| {
                    eprintln!("Error: No value provided after -ls or --limit-and-size parameter.");
                    std::process::exit(1);
                });

                let pair: Vec<&str> = value.split(",").collect();

                if pair.len() != 2 {
                    eprintln!("Error: -ls or --limit-and-size expects two parameters.");
                    std::process::exit(1);
                }

                let limit = pair[0].parse().unwrap_or_else(|_e| {
                    eprintln!("Error: No value provided for limit after -ls or --limit-and-size parameter.");
                    std::process::exit(1);
                });

                let mut size:usize = 0;
                size = parse_size(pair[1]).unwrap_or_else(|_| {
                    eprintln!("Error: Could not parse filter size passed in -ls or --limit-and-size parameter.");
                    std::process::exit(1);
                }) as usize;

                constructions_details.push(ConstructionDetails {
                    construction_type: ConstructionType::LinesAndSize,
                    limit,
                    size,
                    error_rate: 0.0
                });
            }

            // Specifies limit and expected rates of false positives.
            "-le" | "--limit-and-error-rate" => {
                let value = env::args().nth(idx + 1).unwrap_or_else(|| {
                    eprintln!("Error: No value provided after -le or --limit-and-error-rate parameter.");
                    std::process::exit(1);
                });

                let pair : Vec<&str> = value.split(",").collect();

                if pair.len() != 2 {
                    eprintln!("Error: -le or --limit-and-error-rate expects two parameters.");
                    std::process::exit(1);
                }

                let limit = pair[0].parse().unwrap_or_else(|_e| {
                    eprintln!("Error: No value provided for limit after -le or --limit-and-error-rate parameter.");
                    std::process::exit(1);
                });

                let error_rate: f64 = pair[1]
                    .parse()
                    .unwrap_or_else(|_| {
                        eprintln!("Error: Error rate must be number.");
                        std::process::exit(1);
                    });

                if error_rate <= 0.0 || error_rate >= 1.0 {
                    eprintln!("Error: Error rate must be a number greater than 0.0 and less than 1.0. \"{}\" passed.", error_rate);
                    std::process::exit(1);
                }

                constructions_details.push(ConstructionDetails {
                    construction_type: ConstructionType::LinesAndErrorRate,
                    limit,
                    error_rate,
                    size: 0
                });
            }

            // Whether we want to update (write to) Bloom filter files.
            "-w" | "--write" => params.write_mode = true,

            // Will output debug information.
            "-d" | "--debug" => params.debug = true,
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ => (),
        }
    }

    // Checking arguments.

    if file_paths.is_empty() && !params.write_mode {
        // When no paths was given then we're assuming that we work on the memory, so need to enable writing.
        params.write_mode = true;
    }

    if !file_paths.is_empty() {
        eprintln!("Error: Writing to/reading from files is not yet supported.");
        std::process::exit(1);
    }

    if params.write_mode {
        if file_paths.len() > 1 && file_paths.len() != constructions_details.len() {
            eprintln!("Error: Number of passed -le or -ls parameters should be exactly one or match the number of file paths.");
            std::process::exit(1);
        }
    }

    // Building up list of ContainerDetails structures.

    let num_containers = constructions_details.len();

    for idx in 0 .. num_containers {
        params.containers_details.push(ContainerDetails {
            path: format!("<memory #{idx}>"),
            construction_details: constructions_details[idx],
            data_source: DataSource::Memory,
        });
    }

    process(&mut params);
}
