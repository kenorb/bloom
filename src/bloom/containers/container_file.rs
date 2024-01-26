use bloom::containers::container::Container;

struct FileContainer {
    is_acquired: bool,
    num_writes: usize,
    max_writes: usize,
}

impl Container for FileContainer {
    fn acquire(&mut self) {
    }

    fn release(&mut self) {
    }

    fn set(&mut self, value: &String) {
        self.num_writes += 1;
    }

    fn check(&self, value: &String) -> bool {
        return false;
    }

    fn is_full(&self) -> bool {
        return self.num_writes >= self.max_writes;
    }
}

impl FileContainer {
    fn new(items_count: usize, fp_p: f64) -> Self {
        Self {
            is_acquired: false,
            num_writes: 0,
            max_writes: items_count
        }
    }
}