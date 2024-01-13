use std::io::Read;
use bit_set::BitSet;
use bloom::readers_writers::reader_writer::{ReaderWriter, ReaderWriter};

struct MemoryReaderWriter {
    is_acquired: bool,
    bitset: BitSet<bool>,
}

impl ReaderWriter for MemoryReaderWriter {
    fn acquire(&mut self) {
        self.is_acquired = true;
    }
    fn release(&mut self) {
        self.is_acquired = false;
    }

    fn set(&mut self, index: usize) {

    }

    fn get(index: usize) {

    }

    fn new() -> Self {
        MemoryReaderWriter {
            is_acquired: false,
            bitset: BitSet<bool>::new()
        }
    }

}