
use std::fs::OpenOptions;
use std::io::{self, Read, Seek, Write};

pub struct BitSetFile {
    file: std::fs::File,
    num_bits: u64,
}

impl BitSetFile {
    /// Constructor.
    /// # Arguments
    /// * `file_path` -
    /// *  `num_bits` -
    pub fn new(file_path: &str, num_bits: u64) -> Self {
        let num_bytes: u64 = (num_bits + 7) / 8;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path).unwrap_or_else(|err| {
                eprintln!(
                    "Error: Failed to read/write Bloom filter file: {}: {}", file_path, err
                );
                std::process::exit(1);
            }
        );

        // Initialize the file with zeroes
        file.set_len(num_bytes).expect("Cannot initialize bloom filter file size.");

        Self {
            file,
            num_bits,
        }
    }

    /// Reads given bit from file.
    /// # Arguments
    /// * `bit_index` - Index of the bit, e.g., bit 8 means first bit from the second byte (indexing from 0).
    /// # Returns
    /// Result with boolean value from the the given bit index.
    pub fn read_bit(&mut self, bit_index: u64) -> io::Result<bool> {
        let byte_index = bit_index / 8;

        if bit_index >= self.num_bits {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Bit index out of bounds"));
        }

        let mut read_buffer = [0u8];
        self.file.seek(io::SeekFrom::Start(byte_index as u64))?;
        self.file.read_exact(&mut read_buffer)?;

        Ok((read_buffer[0] & (1 << (bit_index % 8))) != 0)
    }

    /// Writes given bit to file.
    /// # Arguments
    /// * `bit_index` - Index of the bit, e.g., bit 8 means first bit from the second byte (indexing from 0).
    /// *     `value` - Value for the bit.
    /// # Returns
    /// Empty result.
    pub fn write_bit(&mut self, bit_index: u64, value: bool) -> io::Result<()> {
        let _byte_index = bit_index / 8;

        if bit_index >= self.num_bits {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Bit index out of bounds"));
        }

        // Read the corresponding byte from the file
        let mut buffer = [0u8];
        self.file.seek(io::SeekFrom::Start(bit_index as u64))?;
        self.file.read_exact(&mut buffer)?;

        // Update the bit in the buffer
        if value {
            buffer[0] |= 1 << (bit_index % 8);
        } else {
            buffer[0] &= !(1 << (bit_index % 8));
        }

        // Write the updated byte back to the file
        self.file.seek(io::SeekFrom::Start(bit_index as u64))?;
        self.file.write_all(&buffer)?;

        // Update the BitSet
        /*
        if value {
            self.bitset.insert(bit_index);
        } else {
            self.bitset.remove(bit_index);
        }
        */

        Ok(())
    }
}