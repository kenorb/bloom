use bloomfilter::Bloom;
use bloom::readers_writers::reader_writer::{ReaderWriter};

pub(crate) struct MemoryReaderWriter {
    is_acquired: bool,
    num_writes: usize,
    filter: Bloom<String>,
}

impl ReaderWriter for MemoryReaderWriter {
    fn acquire(&mut self) {
        self.is_acquired = true;
    }
    fn release(&mut self) {
        self.is_acquired = false;
    }

    fn set(&mut self, value: &String) {
        self.filter.set(value);
    }

    fn check(&self, value: &String) -> bool {
        return self.filter.check(value);
    }
}

impl MemoryReaderWriter {
    pub(crate) fn new(items_count: usize, fp_p: f64) -> Self {
        Self {
            is_acquired: false,
            num_writes: 0,
            filter: Bloom::new_for_fp_rate(items_count, fp_p)
        }
    }
}
