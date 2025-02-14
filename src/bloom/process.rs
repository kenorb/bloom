use std::io::{self, BufRead, BufReader, BufWriter, stdin, StdoutLock, Write};
use memory_stats::memory_stats;
use crate::{Params};
use crate::{DataSource};
use crate::ConstructionType;

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
    let mut line_idx: i64 = 0;

    {
        let mut stdout_lock = BufWriter::with_capacity(BUFFER_CAPACITY, handle);
        let stdin = stdin().lock();
        let mut reader = BufReader::new(stdin);
        let mut buf = Vec::new();

        loop {
            buf.clear();
            let _bytes_read = match reader.read_until(b'\n', &mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Error reading line {}: {}", line_idx, e);
                    continue;
                }
            };

            line_idx += 1;

            // Remove trailing newline if present
            if buf.last() == Some(&b'\n') {
                buf.pop();
            }

            // Create a String if valid UTF-8, otherwise use raw bytes
            match String::from_utf8(buf.clone()) {
                Ok(line) => process_line(&line, params, &mut curr_container_idx, &mut stdout_lock),
                Err(_) => {
                    // Handle invalid UTF-8 by using raw bytes
                    stdout_lock.write_all(&buf).unwrap();
                    stdout_lock.write_all(b"\n").unwrap();
                }
            }
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
    // Step 1: Iterating over containers and checking if value exists in each of them.
    //         If value exists in container, we store (in write mode) the value in the first possible writable
    //         container. In order to find possible container we just skip current container if it's full in a loop.
    //         Special case is for container that is also the current writable container (after finding possible
    //         writable container in a loop). In such case we do check_and_set() to speed up.

    let mut could_write = params.write_mode;
    let mut value_written = false;
    let mut value_found = false;

    // 1. Switching to next writable container.
    if params.write_mode {
        while *curr_writable_container_idx < params.containers.len() && params.containers[*curr_writable_container_idx].is_full() {
            // If current container is full then we advance current writable container index.
            // Note that we could end up with an index out of range. That will mean that we couldn't write items at all.
            if params.debug_internal {
                eprintln!("> #{}: Container is full, we will check another.", *curr_writable_container_idx);
            }
            *curr_writable_container_idx += 1;
        }

        if *curr_writable_container_idx >= params.containers.len() {
            // Current writable container index is out of range. That mean that there is no container that we can write
            // to.
            if params.debug_internal {
                eprintln!("> All containers are full, writing disabled.");
            }
            could_write = false;
        }
        else {
            if params.debug_internal {
                eprintln!("> #{}: Container is not full and is ready to be written to.", *curr_writable_container_idx);
            }
        }
    }

    // 2. Iterating over containers in order to read and maybe write to the same container. If we end up with a matching
    //    value in the container which isn't the writable one then we will write the value in step 3 (outside the loop).
    for (idx, ref mut container) in params.containers.iter_mut().enumerate()
    {
        if could_write && idx == *curr_writable_container_idx {
            // In write mode we could use check_and_set() if current writable container is the one we iterate over.
            value_found = container.check_and_set(line);
            // If value was found then it also was written.
            value_written = value_found;

            if value_found {
                // We found the value and also wrote it into the container. We're advancing to the step 3 in which we
                // will print the value. In step 3 we will not write the value as value_written is now true.
                if params.debug_internal {
                    eprintln!("> #{}: We can write and it's writable container. Value \"{}\" found and written. Advancing to step 3.", idx, line);
                }
                break;
            }
            else {
                // Value wasn't found nor written. Next containers will not be writable. We will just iterate to search
                // for the value and then go to the step 3 in which we may write the value.
                if params.debug_internal {
                    eprintln!("> #{}: We can write and it's writable container. Value \"{}\" not found and not written. Continuing iteration.", idx, line);
                }
                continue;
            }
        }
        else {
            // We can't write, so we fall back to the check().
            value_found = container.check(line);

            if value_found {
                // If value was found then we mark it as already written to not write it again. We can also advance to
                // the step 3.
                if params.debug_internal {
                    eprintln!("> #{}: Value \"{}\" found so we treat is as already written. Advancing to step 3.", idx, line);
                }
                value_written = true;
                break;
            }
            else {
                // Value not found. Continuing iteration.
                if params.debug_internal {
                    eprintln!("> #{}: We can't write. Value \"{}\" not found. Continuing iteration.", idx, line);
                }
                continue;
            }
        }
    }

    // 3. Here we could have three variables which determines what we'll do:
    //    - could_write - If false then we're sure that it's read mode or there's no writable containers.
    //                    If true then we're sure that it's write mode and current writable container is not full and
    //                    is ready to be written to.
    //    - value_found - Whether given line was found in any of the container.
    //    - value_written - Whether given line was written to any of the writable containers.
    //
    if value_found {
        if could_write && !value_written {
            // Value was found in some container, but was not yet written.
            // Note that could_write mean that current writable container is not full and is ready to be written to.
            let curr_writable_container = &mut params.containers[*curr_writable_container_idx];

            if params.debug_internal {
                eprintln!("> #{}: Value \"{}\" found and written in step 3.", *curr_writable_container_idx, line);
            }

            // We're node. Value was found and is now written.
            curr_writable_container.set(line);

            // Marking value as written, so we can do some additional logic later.
            // value_written = true; // Uncomment this if used.
        }
    }

    // 4. Now it's time to print the value. We consider inverse mode.
    if (!value_found && !params.inverse) || (value_found && params.inverse) {
        if !params.silent {
            // Printing the line.
            stdout_lock.write(line.as_bytes()).unwrap();
            stdout_lock.write(b"\n").unwrap();
            if params.debug_internal {
                eprintln!("> Value written: {}", line);
            }
        }
    }
    else {
        if params.debug_internal {
            eprintln!("> Value unmatched: {}", line);
        }
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
        let container_write_level = container.get_write_level();
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

        eprintln!(" - Container {kind_str} \"{}\" with type = {}, size = {}, error rate = {}, limit = {}, binary fill = {} %, line fill = {} %",
                  container_details.path,
                  type_str,
                  container_details.construction_details.size,
                  container_details.construction_details.error_rate,
                  container_details.construction_details.limit,
                  container_usage,
                  container_write_level
        );
    }
    eprintln!();
}
