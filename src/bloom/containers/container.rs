use std::fs::File;
use std::convert::TryFrom;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use byteorder::LittleEndian;

use crate::{ConstructionDetails, ConstructionType};
use crate::{ContainerDetails, DataSource};
use crate::bloom::containers::container_memory_bloom::MemoryContainerBloom;
use crate::bloom::containers::container_memory_xxh::MemoryContainerXXH;

/// Magic value used as first four bytes of container files.
const MAGIC: u32 = 0xB1008811;

pub trait Container
{
    /// Inserts value into the filter.
    fn set(&mut self, value: &String);

    /// Checks whether filter could have given value.
    fn check(&self, value: &String) -> bool;

    /// Checks whether filter could have given value and if no, inserts the value. Returns true if value could have
    /// existed.
    fn check_and_set(&mut self, value: &String) -> bool;

    /// Checks whether container is full, and we should not insert new values.
    fn is_full(&self) -> bool;

    /// Returns construction info used to create this container.
    fn get_container_details(&mut self) -> &mut ContainerDetails;

    /// Returns container fill percentage.
    fn get_usage(&self) -> f32;

    /// Returns container writes percentage.
    fn get_write_level(&self) -> f32 {
        100.0f32 / self.get_num_max_writes() as f32 *  self.get_num_writes() as f32
    }

    // Returns number of writes into the container.
    fn get_num_writes(&self) -> u64;

    // Sets number of writes into the container (initialized when container file is opened).
    fn set_num_writes(&mut self, value: u64);

    // Returns maximum number of allowed writes into the container.
    fn get_num_max_writes(&self) -> u64;

    // Sets maximum number of allowed writes into the container (initialized when container file is opened).
    fn set_num_max_writes(&mut self, value: u64);

    /// Saves (overwrites) container into the file.
    fn save(&mut self) {
        let path = &self.get_container_details().path;

        let mut file = File::create(path).unwrap();

        // Writing magic value.
        file.write_u32::<BigEndian>(MAGIC).unwrap();

        let container_details = self.get_container_details();

        // Writing construction type, e.g., BloomLinesAndSize, XXHLimitAndSize.
        file.write_u8(container_details.construction_details.construction_type as u8).unwrap();

        // Writing size.
        file.write_u64::<LittleEndian>(container_details.construction_details.size as u64).unwrap();

        // Writing limit.
        file.write_u64::<LittleEndian>(container_details.construction_details.limit as u64).unwrap();

        // Writing error rate.
        file.write_f64::<LittleEndian>(container_details.construction_details.error_rate).unwrap();

        // Writing number of written items.
        file.write_u64::<LittleEndian>(self.get_num_writes()).unwrap();

        // Writing maximum number of written items.
        file.write_u64::<LittleEndian>(self.get_num_max_writes()).unwrap();

        // Aligning to 128 bytes, so structure may grow without affecting content.
        for _ in 0 .. 83 {
            file.write_u8(0).unwrap();
        }

        self.save_content(&mut file);

    }

    /// Saves filter data content to the given, already opened for write file.
    fn save_content(&mut self, file: &mut File);

    /// Loads filter data content from the given, already opened file.
    fn load_content(&mut self, file: &mut File);
}

impl dyn Container {
    // Creates container from container details.
    pub fn from_details(container_details: ContainerDetails) -> Box<dyn Container> {
        if matches!(container_details.construction_details.construction_type, ConstructionType::BloomLinesAndErrorRate {..}) {
            return Box::new(MemoryContainerBloom::new_limit_and_error_rate(container_details));
        } else if matches!(container_details.construction_details.construction_type, ConstructionType::BloomLinesAndSize {..}) {
            return Box::new(MemoryContainerBloom::new_limit_and_size(container_details));
        } else if matches!(container_details.construction_details.construction_type, ConstructionType::XXHLimitAndSize {..}) {
            return Box::new(MemoryContainerXXH::new_limit_and_size(container_details));
        } else {
            eprintln!("Internal Error: Construction type not implemented.");
            std::process::exit(1);
        }
    }

    // Creates container from existing file.
    pub fn from_file(path: &String) -> Box<dyn Container> {
        let file = &mut File::open(path).unwrap_or_else(|_| {
            eprintln!("Error: Can't open file \"{}\" for reading!", path);
            std::process::exit(1);
        });

        // Reading magic value.
        let magic = file.read_u32::<BigEndian>().unwrap();

        if magic != MAGIC {
            eprintln!("Error: File \"{}\" is not a bloom filter file!", path);
            std::process::exit(1);
        }

        // Reading construction type, e.g., BloomLinesAndSize, XXHLimitAndSize.
        let construction_type = ConstructionType::try_from(file.read_u8().unwrap()).unwrap();

        // Reading size.
        let size = file.read_u64::<LittleEndian>().unwrap();

        // Reading limit.
        let limit = file.read_u64::<LittleEndian>().unwrap();

        // Reading error rate.
        let error_rate = file.read_f64::<LittleEndian>().unwrap();

        // Reading number of written items.
        let num_writes = file.read_u64::<LittleEndian>().unwrap();

        // Reading maximum number of written items.
        let num_max_writes = file.read_u64::<LittleEndian>().unwrap();

        let construction_details = ConstructionDetails {
            construction_type,
            size,
            limit,
            error_rate
        };

        // Aligning to 128 bytes, so structure may grow without affecting content.
        for _ in 0 .. 83 {
            file.read_u8().unwrap();
        }

        let mut container = <dyn Container>::from_details(ContainerDetails {
            path: path.to_string(),
            construction_details,
            data_source: DataSource::File
        });

        container.set_num_writes(num_writes);

        container.set_num_max_writes(num_max_writes);

        container.load_content(file);

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock implementation for testing
    struct MockContainer {
        value: String,
    }

    impl Container for MockContainer {
        fn set(&mut self, value: &String) {
            self.value = value.clone();
        }

        fn check(&self, value: &String) -> bool {
            self.value == *value
        }

        fn check_and_set(&mut self, value: &String) -> bool {
            let exists = self.check(value);
            if !exists {
                self.set(value);
            }
            exists
        }

        fn is_full(&self) -> bool {
            false
        }

        fn get_container_details(&mut self) -> &mut ContainerDetails {
            unimplemented!("Not needed for this test")
        }

        fn get_usage(&self) -> f32 {
            0.0
        }

        fn get_num_writes(&self) -> u64 {
            0
        }

        fn set_num_writes(&mut self, _value: u64) {}

        fn get_num_max_writes(&self) -> u64 {
            100
        }

        fn set_num_max_writes(&mut self, _value: u64) {}

        fn save_content(&mut self, _file: &mut File) {}

        fn load_content(&mut self, _file: &mut File) {}
    }

    #[test]
    fn test_check_and_set() {
        let mut container = MockContainer {
            value: String::new(),
        };

        let test_value = String::from("test");

        // First check should return false and set the value
        assert!(!container.check_and_set(&test_value));

        // Second check should return true as value exists
        assert!(container.check_and_set(&test_value));
    }

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
