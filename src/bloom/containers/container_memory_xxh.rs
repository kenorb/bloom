use std::fs::File;
use bit_vec::BitVec;
use std::io::{Write, Read, BufWriter};
use bloom::containers::container::{Container};
use xxhash_rust::xxh3::xxh3_64;

use ::{ContainerDetails};

pub(crate) struct MemoryContainerXXH {
    container_details: ContainerDetails,
    is_acquired: bool, // Whether container is in use.
    num_writes: u64, // Number of written keys/values.
    max_writes: u64, // Maximum number of added keys/values.
    bit_vec: BitVec, // Vector of bits used to store keys/values.
    key_bits: u8, // Number of bits used for each key in the slot.
    slot_bits: u8, // Total number of bits used for each slot.
    num_slots: u64, // Total number of slots in the vector of bits.
    num_tries: u64 // Maximum number of lookups when adding/retrieving keys/values.
}

/// Performs input value scaling.
fn remap(value: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
    return out_min + (value - in_min) * (out_max - out_min) / (in_max - in_min);
}

/// Calculates index of the slot where we can insert key which is a part of given hash.
fn calc_slot_index(container: &MemoryContainerXXH, hash: u64) -> u64 {
    remap(hash as f64, 0f64, u64::MAX as f64, 0f64, (container.num_slots - 1) as f64) as u64 % container.num_slots
}

/// Returns u32 made from bit_vec bits of given index range. Note that both indices are inclusive.
fn get_bit_vec_slice(container: &MemoryContainerXXH, slot_bit_from: u64, slot_bit_to: u64) -> u32 {
    let mut result: u32 = 0;
    for i in 0 .. slot_bit_to - slot_bit_from + 1 {
        let bit_value = container.bit_vec.get((slot_bit_from + i) as usize).unwrap();
        if bit_value {
            result |= 1 << i;
        }
    }
    result
}

/// Writes key bits into container. Note that both indices are inclusive.
fn set_bit_vec_slice(container: &mut MemoryContainerXXH, slot_bit_from: u64, slot_bit_to: u64, key: u32) {
    for i in 0 .. slot_bit_to - slot_bit_from + 1 {
        let bit_value = if key & (1 << i) != 0 { true } else { false };
        container.bit_vec.set((slot_bit_from + i) as usize, bit_value);
    }
}

/// Extracts key_bits bits from the hash.
fn get_hash_key_value(container: &MemoryContainerXXH, hash: u64) -> u32 {
    (hash & ((1 << container.key_bits) - 1)) as u32
}

/// Writes key in the given slot index. Marks slot as occupied.
fn write_key(container: &mut MemoryContainerXXH, mut slot_idx: u64, key: u32) {
    slot_idx = slot_idx % container.num_slots;
    // Marking slot as occupied.
    let slot_occupied_bit = slot_idx * container.slot_bits as u64;
    container.bit_vec.set(slot_occupied_bit as usize, true);
    // Writing key into slot.
    let slot_key_bit_from = (slot_idx * container.slot_bits as u64) + 1;
    let slot_key_bit_to = slot_key_bit_from + container.key_bits as u64 - 1; // Inclusive end index.
    set_bit_vec_slice(container, slot_key_bit_from, slot_key_bit_to, key);
    container.num_writes += 1
}

/// Reads key in the given slot index.
fn read_key(container: &MemoryContainerXXH, mut slot_idx: u64) -> u32 {
    slot_idx = slot_idx % container.num_slots;
    // Writing key from slot.
    let slot_key_bit_from = (slot_idx * container.slot_bits as u64) + 1;
    let slot_key_bit_to = slot_key_bit_from + container.key_bits as u64 - 1; // Inclusive end index.
    get_bit_vec_slice(container, slot_key_bit_from, slot_key_bit_to)
}

/// Checks whether slot is in use.
fn get_slot_in_use(container: &MemoryContainerXXH, mut slot_idx: u64) -> bool {
    slot_idx = slot_idx % container.num_slots;
    // Reading first bit of the slot which indicates whether slot is in use.
    container.bit_vec.get((slot_idx * container.slot_bits as u64) as usize).unwrap()
}

/// Tries to insert part of the hash in the first free slot starting from the specified slot index.
/// Returns true if key was found and thus doesn't need to be inserted.
fn insert_key(container: &mut MemoryContainerXXH, slot_idx: u64, hash: u64, num_tries: u64) -> bool {
    // Extracting key_bits bits from the hash.
    let hash_key_value = get_hash_key_value(container, hash);

    // We only search in num_tries consecutive slots.
    for i in 0 .. num_tries {
        // First slot's bit is whether slot is occupied.
        if get_slot_in_use(container, slot_idx + i) {
            // Slot is in use, maybe it's the one we want to write?
            if read_key(container, slot_idx + i) == hash_key_value {
                // Key already found so returning true.
                return true;
            }
            // Slot in use, but key wasn't found, continuing iteration until we find free slot.
            continue;
        }
        // Free slot found, writing key and marking as occupied.
        write_key(container, slot_idx + i, hash_key_value);
        // Key wasn't found so returning false.
        return false;
    }

    // No free slot found nor matching key in consecutive slots, returning false.
    false
}

/// Tries to find key that matches a part of given hash starting from the given slot index.
/// We search for num_tries consecutive keys and then just return true if there was no match.
fn find_key(container: &MemoryContainerXXH, slot_idx: u64, hash: u64, num_tries: u64) -> bool {
    // Extracting key_bits bits from the hash.
    let hash_key_value = get_hash_key_value(container, hash);

    // We only search in num_tries consecutive slots.
    for i in 0 .. num_tries {
        if !get_slot_in_use(container, slot_idx + i) {
            // Slot not in use, so we're sure that there were no matching key.
            return false;
        }

        // We have occupied slot, checking if hash's key matches.
        if read_key(container, slot_idx + i) == hash_key_value {
            // Matching key. Assuming hash was found.
            return true;
        }
    }

    // All slots were occupied, but we didn't find the matching one. Assuming matching key exists.
    true
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
        insert_key(self, slot_idx, hash, self.num_tries);
        self.num_writes += 1;
    }

    /// Checks whether filter could have given value.
    fn check(&self, value: &String) -> bool {
        // Very naive version of check. Just for testing purposes.
        let hash = xxh3_64(value.as_bytes());
        let slot_idx = calc_slot_index(self, hash);
        // We won't use the free_index in read mode.
        return find_key(self, slot_idx, hash, self.num_tries);
    }

    /// Checks whether filter could have given value and if no, inserts the value. Returns true if value could have
    /// existed.
    fn check_and_set(&mut self, value: &String) -> bool {
        let hash = xxh3_64(value.as_bytes());
        let slot_idx = calc_slot_index(self, hash);
        // insert_key() will return whether key was found while trying to insert it.
        return insert_key(self, slot_idx, hash, self.num_tries);
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
    /// Creates instance of bloom filter from given container details.
    pub(crate) fn new_limit_and_size(container_details: ContainerDetails) -> Self {
        let key_bits: u8 = 20;
        let slot_internal_bits: u8 = 1; // We will only store boolean indicating whether slot is occupied.
        Self {
            is_acquired: false,
            num_writes: 0,
            max_writes: container_details.construction_details.limit,
            bit_vec: BitVec::from_elem(container_details.construction_details.size as usize * 8, false),
            key_bits,
            slot_bits: slot_internal_bits + key_bits,
            num_slots: (container_details.construction_details.size * 8) / (slot_internal_bits as u64 + key_bits as u64),
            num_tries: 4,
            container_details,
        }
    }
}
