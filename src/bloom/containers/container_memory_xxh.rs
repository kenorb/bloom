use std::fs::File;
use bit_vec::BitVec;

use std::io::{Write, Read, BufWriter};


use bloom::containers::container::{Container};
use xxhash_rust::xxh3::xxh3_64;

use ::{ContainerDetails};

pub(crate) struct MemoryContainerXXH {
    container_details: ContainerDetails,
    is_acquired: bool,
    num_writes: usize,
    max_writes: usize,
    bitvec: BitVec
}

/// Performs input value scaling.
fn remap(value: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
    return out_min + (value - in_min) * (out_max - out_min) / (in_max - in_min);
}

fn calc_index(container: &MemoryContainerXXH, hash: u64) -> usize {
    let size = container.bitvec.len();
    return remap(hash as f64, 0f64, u64::MAX as f64, 0f64, (size - 1) as f64) as usize;
}

impl Container for MemoryContainerXXH {
    /// Acquires access to the content.
    fn acquire(&mut self) {
        self.is_acquired = true;
    }

    /// Releases access to the content.
    fn release(&mut self) {
        self.is_acquired = false;
    }

    /// Inserts value into the filter.
    fn set(&mut self, value: &String) {
        let hash = xxh3_64(value.as_bytes());
        let base_index = calc_index(self, hash);
        self.bitvec.set(base_index, true);
        self.num_writes += 1;
    }

    /// Checks whether filter could have given value.
    fn check(&self, value: &String) -> bool {
        // Very naive version of check. Just for testing purposes.
        let hash = xxh3_64(value.as_bytes());
        let base_index = calc_index(self, hash);
        return self.bitvec.get(base_index).unwrap();
    }

    /// Checks whether filter could have given value and if no, inserts the value. Returns true if value could have
    /// existed.
    fn check_and_set(&mut self, value: &String) -> bool {
        let hash = xxh3_64(value.as_bytes());
        let base_index = calc_index(self, hash);
        let had_value = self.bitvec.get(base_index).unwrap();
        if !had_value {
            self.bitvec.set(base_index, true);
        }
        return had_value;
    }

    /// Checks whether container is full, and we should not insert new values.
    fn is_full(&self) -> bool {
        return self.num_writes >= self.max_writes;
    }

    /// Returns construction info used to create this container.
    fn get_container_details(&mut self) -> &mut ContainerDetails {
        &mut self.container_details
    }

    /// Saves filter data content to the given, already opened for write file.
    fn save_content(&mut self, file: &mut File) {
        eprintln!("Starting write");
        let mut buf_writer = BufWriter::with_capacity(10000000, file);
        buf_writer.write_all(&self.bitvec.to_bytes()).unwrap();
        eprintln!("Finished write");
    }

    /// Loads filter data content from the given, already opened file.
    fn load_content(&mut self, mut file: &File) {
        let construction_details = &self.get_container_details();

        let mut bytes = Vec::new();
        bytes.reserve_exact(construction_details.construction_details.size * 8);
        file.read_to_end(&mut bytes).unwrap();

        self.bitvec = BitVec::from_bytes(&bytes);
    }
}

impl MemoryContainerXXH {
    pub(crate) fn new_limit_and_size(container_details: ContainerDetails) -> Self {
        Self {
            is_acquired: false,
            num_writes: 0,
            max_writes: container_details.construction_details.limit,
            bitvec: BitVec::from_elem(container_details.construction_details.size * 8, false),
            container_details,
        }
    }
}
