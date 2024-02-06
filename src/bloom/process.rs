use std::io;
use std::io::{BufRead, BufWriter, stdin, StdoutLock, Write};
use memory_stats::memory_stats;
use ::{Params};
use ::{DataSource};
use bloom::containers::container::{Container};
use bloom::containers::container_memory_bloom::{MemoryContainerBloom};
use bloom::containers::container_memory_xxh::{MemoryContainerXXH};
use ConstructionType;


/// Performs Bloom filter tasks.
pub fn process(params: &mut Params) {
    let mut initial_physical_mem: usize = 0;
    let mut initial_virtual_mem: usize = 0;

    if let Some(usage) = memory_stats() {
        initial_physical_mem = usage.physical_mem;
        initial_virtual_mem = usage.virtual_mem;
    }

    if params.debug {
        debug_args(&params);
    }

    // Creating memory containers.
    for (_idx, file) in params.containers_details.iter().enumerate() {
        let container: Box<dyn Container>;

        if matches!(file.data_source, DataSource::Memory {..}) {
            if matches!(file.construction_details.construction_type, ConstructionType::BloomLinesAndErrorRate {..}) {
                container = Box::new(MemoryContainerBloom::new(file.construction_details.limit, file.construction_details.error_rate));
            } else if matches!(file.construction_details.construction_type, ConstructionType::BloomLinesAndSize {..}) {
                container = Box::new(MemoryContainerBloom::new_bitmap_size(file.construction_details.limit, file.construction_details.size));
            } else if matches!(file.construction_details.construction_type, ConstructionType::XXHLimitAndSize {..}) {
                container = Box::new(MemoryContainerXXH::new_bitmap_size(file.construction_details.limit, file.construction_details.size));
            } else {
                eprintln!("Internal Error: Construction type not implemented.");
                std::process::exit(1);
            }
        } else {
            eprintln!("Error: Writing to memory is the only yet supported way.");
            std::process::exit(1);
        }

        params.containers.push(container);
    }

    // Current container index (we always use last one, as previous ones are treated as full).
    let mut curr_container_idx: usize = 0;

    const BUFFER_CAPACITY: usize = 64 * 1024;
    let stdout = io::stdout();
    let handle = stdout.lock();
    let mut stdout_lock = io::BufWriter::with_capacity(BUFFER_CAPACITY, handle);

    for line in stdin().lock().lines() {
        // Processing one line using current container index.
        process_line(&line.unwrap(), params, &mut curr_container_idx, &mut stdout_lock);
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
fn process_line(line: &String, params: &mut Params, curr_container_idx: &mut usize, stdout_lock: &mut BufWriter<StdoutLock>) {
    for (idx, container) in params.containers.iter().enumerate() {
        let exists = container.check(&line);

        if params.debug {
            println!("Input: \"{line}\". Checking container #{idx} - {}", if exists { "String exists" } else { "String does not exist" });
        }

        if exists {
            // Potential match found. We're done.
            return;
        }
    }

    if *curr_container_idx >= params.containers.len() {
        // No more containers to write to. Outputting the line.
        if params.debug {
            println!("> Unmatched (bloom size overflow): \"{}\".", line);
        }
        else {
            if !params.silent {
                stdout_lock.write(line.as_bytes()).unwrap();
                stdout_lock.write(b"\n").unwrap();
            }
        }
        return;
    }

    // No match found in all containers.
    if params.debug {
        println!("> Unmatched: \"{}\".", line);
    }
    else {
        if !params.silent {
            stdout_lock.write(line.as_bytes()).unwrap();
            stdout_lock.write(b"\n").unwrap();
        }
    }

    if !params.write_mode {
        if params.debug {
            println!("Not writing \"{line}\" into container #{} as -w was not passed.", *curr_container_idx);
        }
        return;
    }

    let last_container = &mut params.containers[*curr_container_idx];

    if params.debug {
        println!("Writing \"{line}\" into container #{}...", *curr_container_idx);
    }

    // Writing line into current bloom filter.
    last_container.set(&line);

    if params.debug {
        println!("Written.");
    }

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
            ConstructionType::BloomLinesAndSize => { "(bloom) limit and size" }
            ConstructionType::BloomLinesAndErrorRate => { "(bloom) limit and error-rate" },
            ConstructionType::XXHLimitAndSize => { "(xxhash) limit and error-rate" },
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
