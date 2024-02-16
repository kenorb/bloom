use std::fs::File;
use bit_vec::BitVec;
use std::io::{Write, Read, BufWriter};
use crc32fast::hash;


use bloom::containers::container::{Container};
use xxhash_rust::xxh3::xxh3_64;

use ::{ContainerDetails};

pub(crate) struct MemoryContainerXXH {
    container_details: ContainerDetails,
    is_acquired: bool,
    num_writes: u64,
    max_writes: u64,
    bit_vec: BitVec,
    key_bits: u8,
    slot_bits: u8,
    num_slots: u64
}

/// Performs input value scaling.
fn remap(value: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
    return out_min + (value - in_min) * (out_max - out_min) / (in_max - in_min);
}

fn _calc_index_naive(container: &MemoryContainerXXH, hash: u64) -> usize {
    let size = container.bit_vec.len();
    return remap(hash as f64, 0f64, u64::MAX as f64, 0f64, (size - 1) as f64) as usize;
}

/// Calculates index of the slot where we can insert key which is a part of given hash.
fn calc_slot_index(container: &MemoryContainerXXH, hash: u64) -> u64 {
    let size = container.bit_vec.len();
    remap(hash as f64, 0f64, u64::MAX as f64, 0f64, (container.num_slots - 1) as f64) as u64
}

/// Returns u32 made from bit_vec bits of given index range. Note that both indices are inclusive.
fn get_bit_vec_slice(container: &mut MemoryContainerXXH, slot_bit_from: u64, slot_bit_to: u64) -> u32 {
    let slot_key_bits = &container.bit_vec[slot_bit_from..slot_bit_to + 1];

    // Getting value that the slot holds.
    let mut bits_value = 0u32;
    for &bit in slot_key_bits {
        bits_value = (bits_value << 1) | bit;
    }

    bits_value
}

/// Tries to insert part of the hash in the first free slot starting from the specified slot index.
fn insert_key(container: &mut MemoryContainerXXH, mut slot_idx: u64, hash: u64) -> bool {
    if container.is_full() {
        return false;
    }

    // Extracting key_bits bits from the hash.
    let hash_key_value = hash & ((1 << container.key_bits) - 1);

    // We only search in 10 consecutive slots.
    for i in 0 .. 10 {
        let mut slot_bit_from = (slot_idx + i) * container.slot_bits;

        if (container.bit_vec.get(slot_bit_from as usize)) {
            // Slot is in use, skipping.
            continue;
        }

        let slot_bit_to = (slot_idx + 1) * container.slot_bits + container.slot_bits;
        let slot_key_value = get_bit_vec_slice(container, slot_bit_from, slot_bit_to - 1);

        if (slot_key_value == hash_key_value) {
            // Found hash in the slot!
            return
        }
    }

    false
}

/// Tries to find key that matches a part of given hash starting from the given slot index.
fn find_value(container: &MemoryContainerXXH, mut slot_idx: u64, hash: u64, ref mut free_index: u64) -> bool {
    loop {
        return false
    }

    false
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
        let slot_idx = calc_slot_index(self, hash);
        insert_key(self, slot_idx, hash);
        self.num_writes += 1;
    }

    /// Checks whether filter could have given value.
    fn check(&self, value: &String) -> bool {
        // Very naive version of check. Just for testing purposes.
        let hash = xxh3_64(value.as_bytes());
        let slot_idx = calc_slot_index(self, hash);
        // We won't use the free_index in read mode.
        let _free_index: u64 = 0;
        return find_value(self, slot_idx, hash, _free_index);
    }

    /// Checks whether filter could have given value and if no, inserts the value. Returns true if value could have
    /// existed.
    fn check_and_set(&mut self, value: &String) -> bool {
        let hash = xxh3_64(value.as_bytes());
        let slot_idx = calc_slot_index(self, hash);
        // Free index is the index find_value() will return if it won't find the value. We could use this index to write
        // the key and thus occupy the slot.
        let free_slot_idx: u64 = 0;
        let had_value = find_value(self, slot_idx, hash, free_slot_idx);
        if !had_value {
            insert_key(self, slot_idx, hash);
            self.num_writes += 1;
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

    /// Returns container fill percentage.
    fn get_usage(&self) -> f32 {
        100.0f32 / self.bit_vec.len() as f32 * self.num_writes as f32
    }

    // Returns number of writes into the container.
    fn get_num_writes(&self) -> u64 {
        self.num_writes as u64
    }

    // Sets number of writes into the container (initialized when container file is opened).
    fn set_num_writes(&mut self, value: u64) {
        self.num_writes = value
    }

    // Returns maximum number of allowed writes into the container.
    fn get_num_max_writes(&self) -> u64 {
        self.max_writes as u64
    }

    // Sets maximum number of allowed writes into the container (initialized when container file is opened).
    fn set_num_max_writes(&mut self, value: u64) {
        self.max_writes = value;
    }

    /// Saves filter data content to the given, already opened for write file.
    fn save_content(&mut self, file: &mut File) {
        let mut buf_writer = BufWriter::with_capacity(10000000, file);
        buf_writer.write_all(&self.bit_vec.to_bytes()).unwrap();
    }

    /// Loads filter data content from the given, already opened file.
    fn load_content(&mut self, file: &mut File) {
        let construction_details = &self.get_container_details();
        let mut bytes = Vec::new();
        bytes.reserve_exact(construction_details.construction_details.size as usize);
        file.read_to_end(&mut bytes).unwrap();
        self.bit_vec = BitVec::from_bytes(&bytes);
    }
}

impl MemoryContainerXXH {
    pub(crate) fn new_limit_and_size(container_details: ContainerDetails) -> Self {
        let key_bits: u8 = 4;
        let slot_internal_bits: u8 = 1; // We will only store boolean indicating whether slot is occupied.
        Self {
            is_acquired: false,
            num_writes: 0,
            max_writes: container_details.construction_details.limit,
            bit_vec: BitVec::from_elem(container_details.construction_details.size as usize * 8, false),
            key_bits,
            slot_bits: slot_internal_bits + key_bits,
            num_slots: (container_details.construction_details.size * 8) / (slot_internal_bits as u64 + key_bits as u64),
            container_details,
        }
    }
}
