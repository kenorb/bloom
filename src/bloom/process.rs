use std::io;
use std::io::{BufRead, BufWriter, stdin, StdoutLock, Write};

use memory_stats::memory_stats;
use ::{Params};
use ::{DataSource};

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

    {
        let mut stdout_lock = io::BufWriter::with_capacity(BUFFER_CAPACITY, handle);

        for line in stdin().lock().lines() {
            // Processing one line using current container index.
            process_line(&line.unwrap(), params, &mut curr_container_idx, &mut stdout_lock);
        }
    }

    if params.debug_memory {
        eprintln!();
        eprintln!("[ MEMORY USAGE ]");
        if let Some(usage) = memory_stats() {
            eprintln!("Physical memory used: {:.2} MiB.", (usage.physical_mem - initial_physical_mem) as f64 / 1024.0 / 1024.0);
            eprintln!("Virtual memory used: {:.2} MiB.", (usage.virtual_mem - initial_virtual_mem) as f64 / 1024.0 / 1024.0);
        } else {
            eprintln!("Couldn't get the current memory usage :(");
        }
    }
}

/// Processes a single line.
fn process_line(line: &String, params: &mut Params, curr_writable_container_idx: &mut usize, stdout_lock: &mut BufWriter<StdoutLock>) {
    // Whether line previously existed in any of the containers.
    let mut had_value: bool = false;

    // -w [5 / 1000] [ 1000/1000 ] [10 / 1000]
    //        idx
    //        ^w

    // Whether line was written into the currently writable container (via set() or check_and_set()).
    let mut written: bool = false;

    let mut was_full: bool = false;

    for (idx, ref mut container) in params.containers.iter_mut().enumerate() {
        if params.write_mode && idx > *curr_writable_container_idx {
            // In write mode we only check for the containers up to the currently writable one. Other containers are
            // empty, so there is no sense in checking what's there.
            break;
        }

        if idx == *curr_writable_container_idx {
            // We can only insert to the currently writable container.
            if !container.is_full() {
                // But only if it's not full!
                // Note that in previous iteration we could have found the value, but it wasn't yet written, as
                // container was full.
                if had_value {
                    // We finally insert the value into container.
                    container.set(&line);

                    written = true;
                }
                else {
                    // Previous containers didn't have the value.
                    had_value = container.check_and_set(&line);
                }
                // We're sure that if there were no value then it was written.
                written = true;
            }
            else {
                // If it's full then we fall back into normal check as we can't write into it.
                had_value = container.check(&line);

                was_full = true;
            }
        }
        else {
            // If container is not the currently writable one then we only check if value exists.
            had_value = container.check(&line);
        }

        if params.debug {
            eprintln!("Input: \"{line}\". Checking container #{idx} - {}", if had_value { "String did exist" } else { "String did not exist" });
        }

        if params.inverse {
            if had_value {
                // Value did exist, outputting the line.
                if params.debug {
                    eprintln!("> Inverse matched (string already exist)");
                }

                if !params.silent {
                    stdout_lock.write(line.as_bytes()).unwrap();
                    stdout_lock.write(b"\n").unwrap();
                }
            }
            else {
                if params.debug {
                    eprintln!("> Inverse unmatched (string didn't exist)");
                }
            }
            // If value didn't exist then it was saved, and we can check another line.
            return;
        }
        else if !params.inverse && had_value {
            // Potential match found. We're done.
            return;
        }
    }

    if *curr_writable_container_idx >= params.containers.len() {
        // No more containers to write to. Outputting the line.
        if params.debug {
            eprintln!("> Unmatched (bloom size overflow): \"{}\".", line);
        }

        if !params.silent {
            stdout_lock.write(line.as_bytes()).unwrap();
            stdout_lock.write(b"\n").unwrap();
        }
        return;
    }

    // No match found in all containers.
    if params.debug {
        eprintln!("> Unmatched: \"{}\".", line);
    }

    if !params.silent {
        stdout_lock.write(line.as_bytes()).unwrap();
        stdout_lock.write(b"\n").unwrap();
    }

    if !params.write_mode {
        if params.debug {
            eprintln!("Not writing \"{line}\" into container #{} as -w was not passed.", *curr_writable_container_idx);
        }
        return;
    }

    let curr_writable_container = &mut params.containers[*curr_writable_container_idx];

    if params.debug {
        eprintln!("Writing \"{line}\" into container #{}...", *curr_writable_container_idx);
    }

    if params.write_mode && curr_writable_container.is_full() {
        // If we end up here and current writable container is full then we're
        // We will now use the next container.
        if params.debug {
            eprintln!("Not writing into container #{} as it is full.", *curr_writable_container_idx);
        }
        return;
    }

    // Writing line into current bloom filter.
    curr_writable_container.set(&line);

    if params.debug {
        eprintln!("Written.");
    }
}

fn debug_args(params: &mut Params) {
    eprintln!("[ INPUT ARGUMENTS ]");
    eprintln!(" - debug:      {}", if params.debug { "True" } else { "False" });
    eprintln!(" - write:      {}", if params.write_mode { "True" } else { "False" });
    eprintln!(" - silent:     {}", if params.silent { "True" } else { "False" });
    eprintln!(" - inverse:    {}", if params.inverse { "True" } else { "False" });

    eprintln!();
    eprintln!("[ CONTAINERS ]");
    if params.containers.is_empty() {
        eprintln!(" < No containers added >");
    }

    for (_i, container) in params.containers.iter_mut().enumerate() {
        let container_usage = container.get_usage();
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

        eprintln!(" - Container {kind_str} \"{}\" with type = {}, size = {}, error rate = {}, limit = {}, usage = {} %",
                  container_details.path,
                  type_str,
                  container_details.construction_details.size,
                  container_details.construction_details.error_rate,
                  container_details.construction_details.limit,
                  container_usage
        );
    }
    eprintln!();
}
