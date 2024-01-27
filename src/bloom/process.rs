
use std::io::{BufRead, stdin};
use memory_stats::memory_stats;
use ::{Params};
use ::{DataSource};
use bloom::containers::container::{Container};
use bloom::containers::container_memory::{MemoryContainer};
use ConstructionType;


/// Performs Bloom filter tasks.
pub fn process(params: &mut Params) {
    let mut initial_physical_mem: usize = 0;
    let mut initial_virtual_mem: usize = 0;

    if let Some(usage) = memory_stats() {
        initial_physical_mem = usage.physical_mem;
        initial_virtual_mem = usage.virtual_mem;
    }

    let mut containers: Vec<Box<dyn Container>> = Vec::new();

    if params.debug {
        debug_args(&params);
    }

    // Creating memory containers.
    for (_idx, file) in params.containers_details.iter().enumerate() {
        let container;

        if matches!(file.data_source, DataSource::Memory {..}) {
            if matches!(file.construction_details.construction_type, ConstructionType::LinesAndErrorRate {..}) {
                container = MemoryContainer::new(file.construction_details.limit, file.construction_details.error_rate);
            }
            else if matches!(file.construction_details.construction_type, ConstructionType::LinesAndSize {..}) {
                container = MemoryContainer::new_bitmap_size(file.construction_details.limit, file.construction_details.size);
            }
            else {
                eprintln!("Internal Error: Construction type not implemented.");
                std::process::exit(1);
            }
        }
        else {
            eprintln!("Error: Writing to memory is the only yet supported way.");
            std::process::exit(1);
        }

        containers.push(Box::new(container));
    }

    // Current container index (we always use last one, as previous ones are treated as full).
    let mut curr_container_idx: usize = 0;

    for line in stdin().lock().lines() {
        // Processing one line using current container index.
        process_line(line.unwrap(), params, &mut containers, &mut curr_container_idx);
    }

    if params.debug {
        println!();
        println!("[ MEMORY USAGE ]");
        if let Some(usage) = memory_stats() {
            println!("Physical memory used: {:.2} MiB.", (usage.physical_mem - initial_physical_mem) as f64 / 1024.0 / 1024.0);
            println!(" Virtual memory used: {:.2} MiB.", (usage.virtual_mem - initial_virtual_mem) as f64 / 1024.0 / 1024.0);
        } else {
            println!("Couldn't get the current memory usage :(");
        }
    }
}

/// Processes a single line.
fn process_line(line: String, params: &Params, containers: &mut Vec<Box<dyn Container>>, curr_container_idx: &mut usize) {
    for (idx, container) in containers.iter().enumerate() {
        let exists = container.check(&line);

        if params.debug {
            println!("Input: \"{line}\". Checking container #{idx} - {}", if exists { "String exists" } else { "String does not exist" });
        }

        if exists {
            // Potential match found. We're done.
            return;
        }
    }

    if *curr_container_idx >= containers.len() {
        // No more containers to write to. Outputting the line.
        if params.debug {
            println!("> Unmatched (bloom size overflow): \"{}\".", line);
        }
        else {
            println!("{}", line);
        }
        return;
    }

    // No match found in all containers.
    if params.debug {
        println!("> Unmatched: \"{}\".", line);
    }
    else {
        println!("{}", line);
    }

    if !params.write_mode {
        if params.debug {
            println!("Not writing \"{line}\" into container #{} as -w was not passed.", *curr_container_idx);
        }
        return;
    }

    let last_container = &mut containers[*curr_container_idx];

    if params.debug {
        println!("Writing \"{line}\" into container #{}...", *curr_container_idx);
    }

    // Writing line into current bloom filter.
    last_container.set(&line);

    if last_container.is_full() {
        // We will now use the next container.
        if params.debug {
            println!("Container #{} is now full.", *curr_container_idx);
        }
        *curr_container_idx += 1;
    }
}

fn debug_args(params: &Params) {
    println!("[ INPUT ARGUMENTS ]");
    println!(" - debug:      {}", if params.debug { "True" } else { "False" });
    println!(" - write:      {}", if params.write_mode { "True" } else { "False" });

    println!();
    println!("[ CONTAINERS ]");
    for (_i, file) in params.containers_details.iter().enumerate() {
        let kind_str = match file.data_source {
            DataSource::Memory => { "memory" }
            DataSource::File => { "file" }
        };

        let type_str = match file.construction_details.construction_type {
            ConstructionType::LinesAndSize => { "limit and size" }
            ConstructionType::LinesAndErrorRate => { "limit and error-rate" }
        };

        println!(" - Container {kind_str} \"{}\" with type = {}, size = {}, error rate = {}, limit = {}",
                 file.path,
                 type_str,
                 file.construction_details.size,
                 file.construction_details.error_rate,
                 file.construction_details.limit
        );
    }
    println!();
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
