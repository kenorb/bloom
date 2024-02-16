use std::fs::File;
use std::io::{BufWriter, Write, Read};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use bloomfilter::Bloom;
use bloom::containers::container::{Container};
use ::{ContainerDetails};

pub(crate) struct MemoryContainerBloom {
    container_details: ContainerDetails,
    is_acquired: bool,
    num_writes: u64,
    max_writes: u64,
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
        let had_value = self.filter.check_and_set(value);

        if !had_value {
            self.num_writes += 1;
        }

        had_value
    }

    /// Checks whether container is full, and we should not insert new values.
    fn is_full(&self) -> bool {
        return self.num_writes >= self.max_writes;
    }

    /// Returns construction info used to create this container.
    fn get_container_details(&mut self) -> &mut ContainerDetails {
        &mut self.container_details
    }

    /// Returns container fill percentage.
    fn get_usage(&self) -> f32 {
        100.0f32 / self.filter.bit_vec().len() as f32 * self.num_writes as f32
    }

    // Returns number of writes into the container.
    fn get_num_writes(&self) -> u64 {
        self.num_writes as u64
    }

    // Sets number of writes into the container (initialized when container file is opened).
    fn set_num_writes(&mut self, value: u64) {
        self.num_writes = value;
    }

    // Returns maximum number of allowed writes into the container.
    fn get_num_max_writes(&self) -> u64 {
        self.max_writes as u64
    }

    // Sets maximum number of allowed writes into the container (initialized when container file is opened).
    fn set_num_max_writes(&mut self, value: u64) {
        self.max_writes = value;
    }

    /// Saves filter data content to the given, already opened for write file.
    fn save_content(&mut self, file: &mut File) {
        let mut buf_writer = BufWriter::with_capacity(10000000, file);

        // Writing sip keys.
        let sip_keys = self.filter.sip_keys();
        let (sip_keys_0_0, sip_keys_0_1) = &sip_keys.get(0).unwrap();
        let (sip_keys_1_0, sip_keys_1_1) = &sip_keys.get(1).unwrap();
        buf_writer.write_u64::<LittleEndian>(*sip_keys_0_0).unwrap();
        buf_writer.write_u64::<LittleEndian>(*sip_keys_0_1).unwrap();
        buf_writer.write_u64::<LittleEndian>(*sip_keys_1_0).unwrap();
        buf_writer.write_u64::<LittleEndian>(*sip_keys_1_1).unwrap();

        // Writing bit vec.
        buf_writer.write_all(&self.filter.bit_vec().to_bytes()).unwrap();
    }

    /// Loads filter data content from the given, already opened file.
    fn load_content(&mut self, file: &mut File) {
        let construction_details = self.get_container_details();

        // Reading sip keys.
        let sip_keys_0_0 = file.read_u64::<LittleEndian>().unwrap();
        let sip_keys_0_1 = file.read_u64::<LittleEndian>().unwrap();
        let sip_keys_1_0 = file.read_u64::<LittleEndian>().unwrap();
        let sip_keys_1_1 = file.read_u64::<LittleEndian>().unwrap();

        let sip_keys: [(u64, u64); 2] = [(sip_keys_0_0, sip_keys_0_1), (sip_keys_1_0,sip_keys_1_1)];

        // Reading bit vec.
        let mut bytes = Vec::new();
        bytes.reserve_exact(construction_details.construction_details.size as usize);
        file.read_to_end(&mut bytes).unwrap();

        self.filter = Bloom::from_existing(
            &bytes,
            construction_details.construction_details.size as u64 * 8,
            construction_details.construction_details.limit as u32,
            sip_keys);
    }
}

impl MemoryContainerBloom {
    pub(crate) fn new_limit_and_error_rate(container_details: ContainerDetails) -> Self {
        Self {
            is_acquired: false,
            num_writes: 0,
            max_writes: container_details.construction_details.limit,
            filter: Bloom::new_for_fp_rate(container_details.construction_details.limit as usize, container_details.construction_details.error_rate),
            container_details,

        }
    }
    pub(crate) fn new_limit_and_size(container_details: ContainerDetails) -> Self {
        Self {
            is_acquired: false,
            num_writes: 0,
            max_writes: container_details.construction_details.limit,
            filter: Bloom::new(container_details.construction_details.size as usize, container_details.construction_details.limit as usize),
            container_details,
        }
    }
}
