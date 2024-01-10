pub trait ReaderWriterTrait
{
    /// Acquires access to the content.
    fn acquire();

    /// Releases access to the content.
    fn release();
    fn set(index: usize);
    fn get(index: usize);
}

pub struct ReaderWriter {
    pub is_acquired: bool
}

impl ReaderWriterTrait for ReaderWriter {
    fn acquire() {
        todo!()
    }

    fn release() {
        todo!()
    }

    fn set(index: usize) {
        todo!()
    }

    fn get(index: usize) {
        todo!()
    }
}