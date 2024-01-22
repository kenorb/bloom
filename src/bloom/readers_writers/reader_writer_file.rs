use bloom::readers_writers::reader_writer::ReaderWriter;

struct FileReaderWriter {
    is_acquired: bool,
    num_writes: usize,
}

impl ReaderWriter for FileReaderWriter {
    fn acquire(&mut self) {
    }

    fn release(&mut self) {
    }

    fn set(&mut self, value: &String) {
    }

    fn check(&self, value: &String) -> bool {
        return false;
    }
}

impl FileReaderWriter {
    fn new(items_count: usize, fp_p: f64) -> Self {
        Self {
            is_acquired: false,
            num_writes: 0,
        }
    }
}