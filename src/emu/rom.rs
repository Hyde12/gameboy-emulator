use std::{fs, io::Error};

const EXT_RAM_SIZE: usize = 8192;

pub struct ROMData {
    pub data: Vec<u8>,
    pub title: String,
    pub cgb_flag: Option<u8>,
    pub checksum_verified: bool,
    pub external_ram: [u8; EXT_RAM_SIZE],
    checksum: u8,
}

impl ROMData {
    pub fn new() -> Self {
        ROMData {
            data: Vec::new(),
            title: String::new(),
            cgb_flag: None,
            checksum_verified: false,
            external_ram: [0; EXT_RAM_SIZE],
            checksum: 0,
        }
    }

    pub fn load_rom(&mut self, path: String) -> Result<(), Error> {
        self.data = fs::read(path)?;

        Ok(())
    }
}
