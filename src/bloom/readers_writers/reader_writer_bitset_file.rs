use bit_set::BitSet;
use std::fs::OpenOptions;
use std::io::{self, Read, Seek, Write};

pub struct BitSetFile {
    file: std::fs::File,
    bitset: BitSet,
}

impl BitSetFile {
    pub fn new(file_path: &str, size: usize) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)?;

        // Initialize the file with zeroes
        file.set_len(size as u64)?;

        Ok(Self {
            file,
            bitset: BitSet::with_capacity(size),
        })
    }

    pub fn read_bit(&mut self, index: usize) -> io::Result<bool> {
        if index >= self.bitset.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Index out of bounds",
            ));
        }

        // Read the corresponding byte from the file
        let mut buffer = [0u8];
        self.file.seek(io::SeekFrom::Start(index as u64))?;
        self.file.read_exact(&mut buffer)?;

        Ok((buffer[0] & (1 << (index % 8))) != 0)
    }

    pub fn write_bit(&mut self, index: usize, value: bool) -> io::Result<()> {
        if index >= self.bitset.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Index out of bounds",
            ));
        }

        // Read the corresponding byte from the file
        let mut buffer = [0u8];
        self.file.seek(io::SeekFrom::Start(index as u64))?;
        self.file.read_exact(&mut buffer)?;

        // Update the bit in the buffer
        if value {
            buffer[0] |= 1 << (index % 8);
        } else {
            buffer[0] &= !(1 << (index % 8));
        }

        // Write the updated byte back to the file
        self.file.seek(io::SeekFrom::Start(index as u64))?;
        self.file.write_all(&buffer)?;

        // Update the BitSet
        if value {
            self.bitset.insert(index);
        } else {
            self.bitset.remove(index);
        }

        Ok(())
    }
}