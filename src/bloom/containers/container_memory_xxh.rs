use std::fs::File;
use bit_vec::BitVec;
use byteorder::{LittleEndian};
use std::io::{Write, Result, Read, BufWriter};
use bit_set::BitSet;

use bloom::containers::container::{Container};
use xxhash_rust::xxh3::xxh3_64;
use ::{ConstructionDetails};
use ::{ContainerDetails, DataSource};

pub(crate) struct MemoryContainerXXH {
    container_details: ContainerDetails,
    is_acquired: bool,
    num_writes: usize,
    max_writes: usize,
    bitvec: BitVec,
    hash_bits: usize
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
        let base_index = hash % self.bitvec.len() as u64;
        self.bitvec.set(base_index as usize, true);
        self.num_writes += 1;
    }

    /// Checks whether filter could have given value.
    fn check(&self, value: &String) -> bool {
        // Very naive version of check. Just for testing purposes.
        let hash = xxh3_64(value.as_bytes());
        let base_index = hash % self.bitvec.len() as u64;
        return self.bitvec.get(base_index as usize).unwrap();
    }

    /// Checks whether filter could have given value and if no, inserts the value. Returns true if value could have
    /// existed.
    fn check_and_set(&mut self, value: &String) -> bool {
        let hash = xxh3_64(value.as_bytes());
        let base_index = hash % self.bitvec.len() as u64;
        let had_value = self.bitvec.get(base_index as usize).unwrap();
        if !had_value {
            self.bitvec.get(base_index as usize).unwrap();
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
        println!("Starting write");
        let mut buf_writer = BufWriter::with_capacity(10000000, file);
        buf_writer.write_all(&self.bitvec.to_bytes()).unwrap();
        println!("Finished write");
    }

    /// Loads filter data content from the given, already opened file.
    fn load_content(&mut self, mut file: &File) {
        let construction_details = &self.get_container_details();

        let mut bytes = Vec::new();
        bytes.reserve_exact(construction_details.construction_details.size);
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
            bitvec: BitVec::from_elem(container_details.construction_details.size, false),
            hash_bits: Self::num_bits_required(container_details.construction_details.size),
            container_details,
        }
    }

    fn num_bits_required(value: usize) -> usize {
        if value == 0 {
            // If the value is 0, it still requires at least 1 bit to represent.
            return 1;
        }

        let mut num_bits = 0;
        let mut n = value;

        while n != 0 {
            num_bits += 1;
            n >>= 1;
        }

        num_bits
    }
}
