use std::fs::{self, File};
use std::path::Path;

use crate::emu::RomError;

pub fn read_rom_header<P: AsRef<Path>>(
    path: P,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    // let mut file = match File::open(path) {
    //     Ok(file) => file,
    //     Err(e) => panic!("File couldn't be opened: {}", e),
    // };

    let rom_bytes = fs::read(path)?;

    match &rom_bytes.get(0x0100..0x014F) {
        Some(s) => {
            print!("{:?}", s);
        }
        None => return Err(Box::new(RomError::InvalidRom)),
    };

    return Ok(());
}
