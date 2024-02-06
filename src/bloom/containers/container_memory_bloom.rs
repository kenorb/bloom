extern crate bloomfilter;

use self::bloomfilter::Bloom;
use bloom::containers::container::{Container};

pub(crate) struct MemoryContainerBloom {
    is_acquired: bool,
    num_writes: usize,
    max_writes: usize,
    filter: Bloom<String>,
}

impl Container for MemoryContainerBloom {
    fn acquire(&mut self) {
        self.is_acquired = true;
    }
    fn release(&mut self) {
        self.is_acquired = false;
    }

    fn set(&mut self, value: &String) {
        self.filter.set(value);
        self.num_writes += 1;
    }

    fn check(&self, value: &String) -> bool {
        return self.filter.check(value);
    }

    fn is_full(&self) -> bool {
        return self.num_writes >= self.max_writes;
    }
}

impl MemoryContainerBloom {
    pub(crate) fn new(items_count: usize, fp_p: f64) -> Self {
        Self {
            is_acquired: false,
            num_writes: 0,
            max_writes: items_count,
            filter: Bloom::new_for_fp_rate(items_count, fp_p)
        }
    }
    pub(crate) fn new_bitmap_size(items_count: usize, bitmap_size: usize) -> Self {
        Self {
            is_acquired: false,
            num_writes: 0,
            max_writes: items_count,
            filter: Bloom::new(bitmap_size, items_count)
        }
    }
}
