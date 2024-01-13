use bloom::readers_writers::reader_writer::ReaderWriter;


struct FileReaderWriter {
    is_acquired: bool,
}

impl ReaderWriter for FileReaderWriter {
    fn new(path: String) -> Self {
        Self {
            is_acquired: false
        }
    }

    fn acquire() {
    }

    fn release() {
    }

    fn set(index: usize) {
    }

    fn get(index: usize) {

    }
}