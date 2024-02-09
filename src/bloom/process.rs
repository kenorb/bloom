use std::io;
use std::io::{BufRead, BufWriter, stdin, stdout, StdoutLock, Write};
use std::ops::Deref;
use memory_stats::memory_stats;
use ::{Params};
use ::{bloom, DataSource};
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
        debug_args(params);
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
fn process_line(line: &String, params: &mut Params, curr_writable_container_idx: &mut usize, stdout_lock: &mut BufWriter<StdoutLock>) {
    // Whether line previously existed in any of the containers.
    let mut had_value = false;

    // Whether line was written into the currently writable container (via check_and_set()).
    let mut did_set = false;

    for (idx, ref mut container) in params.containers.iter_mut().enumerate() {
        if params.write_mode && idx < *curr_writable_container_idx {
            // In write mode we only check for the containers up to the currently writable one. Other containers are
            // empty, so there is no sense in checking what's there.
            break;
        }

        if idx == *curr_writable_container_idx {
            // We can only insert to the currently writable container.
            if !container.is_full() {
                // But only if it's not full!
                had_value = container.check_and_set(&line);
                // We're sure that if there were no value then it was written.
                did_set = true;
            }
            else {
                // If it's full then we fall back into normal check as we can't write into it.
                had_value = container.check(&line);
            }
        }
        else {
            // If container is not the currently writable one then we only check if value exists.
            had_value = container.check(&line);
        }

        if params.debug {
            println!("Input: \"{line}\". Checking container #{idx} - {}", if had_value { "String exists" } else { "String does not exist" });
        }

        if had_value {
            // Potential match found. We're done.
            return;
        }
    }

    if *curr_writable_container_idx >= params.containers.len() {
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
            println!("Not writing \"{line}\" into container #{} as -w was not passed.", *curr_writable_container_idx);
        }
        return;
    }

    let mut last_container = &mut params.containers[*curr_writable_container_idx];

    if params.debug {
        println!("Writing \"{line}\" into container #{}...", *curr_writable_container_idx);
    }

    // Writing line into current bloom filter.
    last_container.set(&line);

    if params.debug {
        println!("Written.");
    }

    if last_container.is_full() {
        // We will now use the next container.
        if params.debug {
            println!("Container #{} is now full.", *curr_writable_container_idx);
        }
        *curr_writable_container_idx += 1;
    }
}

fn debug_args(params: &mut Params) {
    println!("[ INPUT ARGUMENTS ]");
    println!(" - debug:      {}", if params.debug { "True" } else { "False" });
    println!(" - write:      {}", if params.write_mode { "True" } else { "False" });

    println!();
    println!("[ CONTAINERS ]");
    if params.containers.is_empty() {
        println!(" < No containers added >");
    }

    for (_i, container) in params.containers.iter_mut().enumerate() {
        let container_details = container.get_container_details();

        let kind_str = match container_details.data_source {
            DataSource::Memory => { "memory" }
            DataSource::File => { "file" }
        };

        let type_str = match container_details.construction_details.construction_type {
            ConstructionType::BloomLinesAndSize => { "(bloom) limit and size" }
            ConstructionType::BloomLinesAndErrorRate => { "(bloom) limit and error-rate" },
            ConstructionType::XXHLimitAndSize => { "(xxhash) limit and error-rate" },
        };

        println!(" - Container {kind_str} \"{}\" with type = {}, size = {}, error rate = {}, limit = {}",
                 container_details.path,
                 type_str,
                 container_details.construction_details.size,
                 container_details.construction_details.error_rate,
                 container_details.construction_details.limit
        );
    }
    println!();
}
