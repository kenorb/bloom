pub trait ReaderWriter
{
    /// Acquires access to the content.
    fn acquire(&mut self);

    /// Releases access to the content.
    fn release(&mut self);

    fn set(&mut self, index: usize);
    fn get(index: usize);
    fn new() -> Self;
}
