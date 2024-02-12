extern crate bloomfilter;

use std::fs::File;
use byteorder::{LittleEndian, WriteBytesExt};
use self::bloomfilter::Bloom;
use bloom::containers::container::{Container};
use ::{ContainerDetails};

pub(crate) struct MemoryContainerBloom {
    container_details: ContainerDetails,
    is_acquired: bool,
    num_writes: usize,
    max_writes: usize,
    filter: Bloom<String>,
}

impl Container for MemoryContainerBloom {
    /// Acquires access to the content.
    fn acquire(&mut self) {
        self.is_acquired = true;
    }

    /// Releases access to the content.
    fn release(&mut self) {
        self.is_acquired = false;
    }

    /// Inserts value into the filter.
    fn set(&mut self, value: &String) {
        self.filter.set(value);
        self.num_writes += 1;
    }

    /// Checks whether filter could have given value.
    fn check(&self, value: &String) -> bool {
        return self.filter.check(value);
    }

    /// Checks whether filter could have given value and if no, inserts the value. Returns true if value could have
    /// existed.
    fn check_and_set(&mut self, value: &String) -> bool {
        return self.filter.check_and_set(value);
    }

    /// Checks whether container is full, and we should not insert new values.
    fn is_full(&self) -> bool {
        return self.num_writes >= self.max_writes;
    }

    /// Returns construction info used to create this container.
    fn get_container_details(&mut self) -> &mut ContainerDetails {
        &mut self.container_details
    }

    /// Saves filter data content to the given, already opened for write file.
    fn save_content(&mut self, file: &mut File) {
        file.write_u32::<LittleEndian>(0xFFEEDDBB).unwrap();
    }

    /// Loads filter data content from the given, already opened file.
    fn load_content(&mut self, _file: &File) {

    }
}

impl MemoryContainerBloom {
    pub(crate) fn new_limit_and_error_rate(container_details: ContainerDetails) -> Self {
        Self {
            is_acquired: false,
            num_writes: 0,
            max_writes: container_details.construction_details.limit,
            filter: Bloom::new_for_fp_rate(container_details.construction_details.limit, container_details.construction_details.error_rate),
            container_details,

        }
    }
    pub(crate) fn new_limit_and_size(container_details: ContainerDetails) -> Self {
        Self {
            is_acquired: false,
            num_writes: 0,
            max_writes: container_details.construction_details.limit,
            filter: Bloom::new(container_details.construction_details.size, container_details.construction_details.limit),
            container_details,
        }
    }
}
