use std::{fs, io::Error};

const EXT_RAM_SIZE: usize = 8192;

pub struct ROMData {
    pub data: Vec<u8>,
    pub boot_rom_data: Vec<u8>,
    pub title: String,
    pub cgb_flag: Option<u8>,
    pub checksum_verified: bool,
    pub external_ram: [u8; EXT_RAM_SIZE],
    pub boot_rom_finished: bool,
    checksum: u8,
}

impl ROMData {
    pub fn new() -> Self {
        ROMData {
            data: Vec::new(),
            boot_rom_data: Vec::new(),
            title: String::new(),
            cgb_flag: None,
            checksum_verified: false,
            external_ram: [0; EXT_RAM_SIZE],
            checksum: 0,
            boot_rom_finished: false,
        }
    }

    pub fn load_rom(&mut self, path: String) -> Result<(), Error> {
        self.boot_rom_data = fs::read("dmg_rom.bin")?;
        self.data = fs::read(path)?;

        Ok(())
    }
}
