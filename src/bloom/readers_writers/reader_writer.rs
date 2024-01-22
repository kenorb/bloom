pub trait ReaderWriter
{
    /// Acquires access to the content.
    fn acquire(&mut self);

    /// Releases access to the content.
    fn release(&mut self);

    fn set(&mut self, value: &String);
    fn check(&self, value: &String) -> bool;
}
