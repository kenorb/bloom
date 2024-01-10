use bloom::readers_writers::reader_writer::ReaderWriterTrait;


struct FileReaderWriter {
}

impl ReaderWriterTrait for FileReaderWriter {
    fn new(path: String) -> Self {
        FileReaderWriter {
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