use bit_vec::BitVec;

use bloom::containers::container::{Container};
use xxhash_rust::xxh3::xxh3_64;

pub(crate) struct MemoryContainerXXH {
    is_acquired: bool,
    num_writes: usize,
    max_writes: usize,
    bitset: BitVec,
    hash_bits: usize
}

impl Container for MemoryContainerXXH {
    fn acquire(&mut self) {
        self.is_acquired = true;
    }
    fn release(&mut self) {
        self.is_acquired = false;
    }

    fn set(&mut self, value: &String) {
        let hash = xxh3_64(value.as_bytes());
        let base_index = hash % self.bitset.len() as u64;
        self.bitset.set(base_index as usize, true);
        self.num_writes += 1;
    }

    fn check(&self, value: &String) -> bool {
        // Very naive version of check. Just for testing purposes.
        let hash = xxh3_64(value.as_bytes());
        let base_index = hash % self.bitset.len() as u64;
        return self.bitset.get(base_index as usize).unwrap();
    }

    fn is_full(&self) -> bool {
        return self.num_writes >= self.max_writes;
    }
}

impl MemoryContainerXXH {
    pub(crate) fn new_bitmap_size(items_count: usize, bitmap_size: usize) -> Self {
        Self {
            is_acquired: false,
            num_writes: 0,
            max_writes: items_count,
            bitset: BitVec::from_elem(bitmap_size, false),
            hash_bits: Self::num_bits_required(bitmap_size)
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
