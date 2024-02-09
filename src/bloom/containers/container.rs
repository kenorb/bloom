use std::fs::File;
use std::io::{Cursor, Write};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use byteorder::LittleEndian;
use ::{ConstructionDetails, ConstructionType};
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use bloom::containers::container_memory_bloom::MemoryContainerBloom;
use bloom::containers::container_memory_xxh::MemoryContainerXXH;
use ::{ContainerDetails, DataSource};

/// Magic value used as first four bytes of container files.
const MAGIC: u32 = 0xB1008811;

pub trait Container
{
    /// Acquires access to the content.
    fn acquire(&mut self);

    /// Releases access to the content.
    fn release(&mut self);

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

    /// Saves (overwrites) container into the file.
    fn save(&mut self) {
        let path = &self.get_container_details().path;

        println!("Saving container into \"{path}\"...");

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

        // Aligning to 128 bytes, so structure may grow without affecting content.
        for _ in 0 .. 99 {
            file.write_u8(0).unwrap();
        }

        self.save_content(&mut file);

    }

    /// Saves filter data content to the given, already opened for write file.
    fn save_content(&mut self, file: &mut File);

    /// Loads filter data content from the given, already opened file.
    fn load_content(&mut self, file: &File);
}

impl Container {
    // Creates container from container details.
    pub fn from_details(container_details: ContainerDetails) -> Box<Container> {
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
    pub fn from_file(path: &String) -> Box<Container> {
        println!("Creating container from file \"{path}\"...");

        let mut file = File::open(path).unwrap_or_else(|_| {
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
        let size = file.read_u64::<LittleEndian>().unwrap() as usize;

        // Reading limit.
        let limit = file.read_u64::<LittleEndian>().unwrap() as usize;

        // Reading error rate.
        let error_rate = file.read_f64::<LittleEndian>().unwrap();

        let construction_details = ConstructionDetails {
            construction_type,
            size,
            limit,
            error_rate
        };

        // Aligning to 128 bytes, so structure may grow without affecting content.
        for _ in 0 .. 99 {
            file.read_u8().unwrap();
        }

        let mut container = Container::from_details(ContainerDetails {
            path: path.to_string(),
            construction_details,
            data_source: DataSource::File
        });

        container.load_content(&file);

        container
    }
}
