extern crate bit_set;
extern crate bit_vec;
extern crate crc32fast;
extern crate parse_size;
extern crate memory_stats;
extern crate xxhash_rust;
extern crate byteorder;
extern crate num_enum;

mod bloom {
    pub mod containers;
    pub mod process;
}

use std::{env};
use std::cmp::max;
use std::fmt::format;
use std::io::{BufRead, Write};
use std::path::Path;
use num_enum::TryFromPrimitive;
use parse_size::parse_size;
use bloom::containers::container::Container;
use bloom::process::process;

#[derive(Copy, Clone)]
enum DataSource {
    Memory,
    File
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u8)]
enum ConstructionType {
    // -bls NUM,NUM[UNIT]
    BloomLinesAndSize,
    // -ble NUM,NUM
    BloomLinesAndErrorRate,
    // -xs NUM
    XXHLimitAndSize,
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
    containers: Vec<Box<dyn Container>>,
    silent: bool
}

fn print_help() {
    // -------------------------------------------------------------------------------------------------------------------------------
    println!("Bloom Filter Command Line Utility");
    println!();
    println!("USAGE:");
    println!("  bloom_filter [OPTIONS]");
    println!();
    println!("DEFAULT BEHAVIOR:");
    println!();
    println!("  When ran without options, one 1Gb xxHash-based with 1M write limit (-xls 1M,1Gb) memory container will be used.");
    println!();
    println!("OPTIONS:");
    println!();
    println!("  -f,   --file FILE                            Specifies Bloom filter file. You may specify multiple files.");
    println!();
    println!("  -w,   --write                                Creates an empty Bloom filter file or updates an existing one.");
    println!();
    println!("  -xls,  --xxh-limit-and-size NUM[UNIT]        Uses xxHash filter. First number limits the number of lines to write into");
    println!("                                               the Bloom filter for each file. Second number specifies Bloom filter size");
    println!("                                               in bytes or given unit. Use -xls once to specify settings for all files");
    println!("                                               or use it multiple times for each file.");
    println!();
    println!("  -bls,  --bloom-lines-and-size NUM,NUM[UNIT]  Uses bloom filter. First number limits the number of lines to write into");
    println!("                                               the Bloom filter for each. file. Second number specifies Bloom filter");
    println!("                                               size in bytes or given unit. Use -bls once to specify settings for all");
    println!("                                               files or use it multiple times for each file.");
    println!();
    println!("  -ble, --bloom-lines-and-error-rate NUM,NUM   Uses bloom filter. First number limits the number of lines to write into");
    println!("                                               the Bloom filter for each file. Second number specifies wanted error rate");
    println!("                                               for the given file (> 0 and < 1). Use -ble once to specify settings for");
    println!("                                               all files or use it multiple times for each file.");
    println!();
    println!("  -d,  --debug                                 Will output debug information.");
    println!();
    println!("  -h,  --help                                  Prints help and usage information.");
    println!();
    println!("  -s,  --silent                                Performs processing but doesn't output anything except -d debug info.");
    println!();
    println!("EXAMPLES:");
    println!();
    println!("  - Will use and write two bloom filter files with maximum of 10 lines and 0.01 error rate each file. All other lines");
    println!("    will not be stored in the files:");
    println!("  $ bloom_filter  -w  -f file1.blf  -f file2.blf  -le 10,0.01  < input.txt");
    println!();
    println!("  - Will use bloom filter in memory and maximum of 10 lines of input for the filter having 100MiB in size.");
    println!("  $ bloom_filter  -bls 10,100MiB  < input.txt");
}

fn main() {
    let mut params = Params {
        debug: false,
        containers_details: vec![],
        write_mode: false,
        containers: Vec::new(),
        silent: false
    };

    // List of passed file paths.
    let mut file_paths: Vec<String> = vec![];

    // List of passed construction details (pairs of limit and error rate or size).
    let mut constructions_details: Vec<ConstructionDetails> = vec![];

    // Parses file arguments from command line. File construction options will be parsed later and file structs will be
    // filled accordingly.
    let mut idx = 1;

    loop {
        if idx >= env::args().len() {
            break;
        }

        let arg: String = env::args().nth(idx).unwrap();
        match arg.as_str() {
            // File output path. Could be passed multiple times.
            "-f" | "--file" => {
                let file_path = env::args().nth(idx + 1).unwrap_or_else(|| {
                    eprintln!("Error: No file path provided after -f or --file parameter.");
                    std::process::exit(1);
                });

                file_paths.push(file_path);

                idx += 1;
            },

            // Specified limit and size of the XXHash filter file in given unit.
            "-xls" | "--xxh-limit-and-size" => {
                let value = env::args().nth(idx + 1).unwrap_or_else(|| {
                    eprintln!("Error: No value provided after -xls or --xxh-limit-and-size parameter.");
                    std::process::exit(1);
                });

                let pair: Vec<&str> = value.split(",").collect();

                if (pair.len() != 2) {
                    eprintln!("Error: -xls or --xxh-limit-and-size expects two parameters.");
                    std::process::exit(1);
                }

                let limit = parse_size(pair[0]).unwrap_or_else(|_| {
                    eprintln!("Error: Could not parse limit passed in -xls or --xxh-limit-and-size parameter.");
                    std::process::exit(1);
                }) as usize;

                let size = parse_size(pair[1]).unwrap_or_else(|_| {
                    eprintln!("Error: Could not parse filter size passed in -xls or --xxh-limit-and-size parameter.");
                    std::process::exit(1);
                }) as usize;

                constructions_details.push(ConstructionDetails {
                    construction_type: ConstructionType::XXHLimitAndSize,
                    limit,
                    size,
                    error_rate: 0.0
                });

                idx += 1;
            }

            // Specified limit and size of the Bloom filter file in given unit.
            "-bls" | "--bloom-limit-and-size" => {
                let value = env::args().nth(idx + 1).unwrap_or_else(|| {
                    eprintln!("Error: No value provided after -bls or --bloom-limit-and-size parameter.");
                    std::process::exit(1);
                });

                let pair: Vec<&str> = value.split(",").collect();

                if (pair.len() != 2) {
                    eprintln!("Error: -bls or --bloom-limit-and-size expects two parameters.");
                    std::process::exit(1);
                }

                let limit = parse_size(pair[0]).unwrap_or_else(|_| {
                    eprintln!("Error: Could not parse limit passed in -xls or --xxh-limit-and-size parameter.");
                    std::process::exit(1);
                }) as usize;

                let size = parse_size(pair[1]).unwrap_or_else(|_| {
                    eprintln!("Error: Could not parse filter size passed in -bls or --bloom-limit-and-size parameter.");
                    std::process::exit(1);
                }) as usize;

                constructions_details.push(ConstructionDetails {
                    construction_type: ConstructionType::BloomLinesAndSize,
                    limit,
                    size,
                    error_rate: 0.0
                });

                idx += 1;
            }

            // Specifies limit and expected rates of false positives.
            "-ble" | "--bloom-limit-and-error-rate" => {
                let value = env::args().nth(idx + 1).unwrap_or_else(|| {
                    eprintln!("Error: No value provided after -ble or --bloom-limit-and-error-rate parameter.");
                    std::process::exit(1);
                });

                let pair : Vec<&str> = value.split(",").collect();

                if (pair.len() != 2) {
                    eprintln!("Error: -ble or --bloom-limit-and-error-rate expects two parameters.");
                    std::process::exit(1);
                }

                let limit = parse_size(pair[0]).unwrap_or_else(|_| {
                    eprintln!("Error: Could not parse limit passed in -xls or --xxh-limit-and-size parameter.");
                    std::process::exit(1);
                }) as usize;

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
                    construction_type: ConstructionType::BloomLinesAndErrorRate,
                    limit,
                    error_rate,
                    size: 0
                });

                idx += 1;
            }

            // Whether we want to update (write to) Bloom filter files.
            "-w" | "--write" => params.write_mode = true,

            // Will output debug information.
            "-d" | "--debug" => params.debug = true,

            // Silent mode.
            "-s" | "--silent" => params.silent = true,

            // Help.
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Error: Invalid parameter passed: \"{}\".", arg);
                std::process::exit(1);
            },
        }

        idx += 1;
    }

    // Checking arguments.

    if file_paths.is_empty() && !params.write_mode {
        // When no paths were given then we're assuming that we work on the memory, so need to enable writing.
        params.write_mode = true;
    }

    if !file_paths.is_empty() && constructions_details.len() > 1 && constructions_details.len() != file_paths.len() {
        eprintln!("Error: Number of passed -xls / -bls / -ble parameters should be exactly zero or one or match the number of file paths.");
        std::process::exit(1);
    }

    if constructions_details.is_empty() {
        // Adding default xxHash memory containers (one or number of file paths passed).
        let num_containers = max(1, file_paths.len());
        for idx in 0 .. num_containers {
            params.containers.push(Container::from_details(ContainerDetails {
                path: if file_paths.is_empty()  { format!("memory.{idx}.out") } else { file_paths[idx].to_string() },
                construction_details: ConstructionDetails {
                    size: parse_size("1Gb").unwrap() as usize,
                    error_rate: 0.0,
                    limit: parse_size("1M").unwrap() as usize,
                    construction_type: ConstructionType::XXHLimitAndSize
                },
                data_source: if file_paths.is_empty() { DataSource::Memory } else { DataSource::File },
            }));
        }
    }

    if !file_paths.is_empty() {
        // Adding file containers.
        for (idx, ref mut construction_details) in constructions_details.iter_mut().enumerate() {
            let path = file_paths[idx].to_string();
            if Path::new(&path).exists() {
                // Creating container from existing file. Input parameters will be overridden by those inside file's
                // header.
                params.containers.push(Container::from_file(&path));
            }
            else {
                params.containers.push(Container::from_details(ContainerDetails {
                    path: path,
                    construction_details: **construction_details,
                    data_source: DataSource::File,
                }));
            }
        }
    }
    else if !constructions_details.is_empty() {
        // Adding memory containers.
        for (idx, ref mut construction_details) in constructions_details.iter_mut().enumerate() {
            params.containers.push(Container::from_details(ContainerDetails {
                path: format!("memory.{idx}.blm"),
                construction_details: **construction_details,
                data_source: DataSource::Memory,
            }));
        }
    }

    process(&mut params);

    if params.write_mode {
        // Writing file containers.
        for (_i, container) in params.containers.iter_mut().enumerate() {
            container.save();
        }
    }
}
