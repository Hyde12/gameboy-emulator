use std::fs::{self};
use std::path::Path;

use crate::emu::RomError;

pub fn read_rom_header<P: AsRef<Path>>(
    path: P,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    // Efficiency (if just gathering header) should go straight to it and not read the whole file
    match fs::read(path)?.get(0x0100..=0x014F) {
        Some(s) => {
            // REFERENCE: https://gbdev.io/pandocs/The_Cartridge_Header.html

            let start_hex = 0x0100; // Subtract from memory places as the original array was cut

            let title = match s.get(0x0134 - start_hex..0x0143 - start_hex) { //TODO: change this so that it checks 0x01043 first before assuming title length
                Some(title) => {
                    println!("{:?}", title);
                    str::from_utf8(title)?.replace("\0", "")
                }
                None => return Err(Box::new(RomError::InvalidRom)),
            };

            { // Checksum Validation
                let checksum = match s.get(0x014D - start_hex) {
                    Some(checksum) => checksum,
                    None => return Err(Box::new(RomError::InvalidRom)),
                };

                let checksum_total = match s.get(0x0134 - start_hex..=0x014C - start_hex) {
                    Some(s) => s
                        .iter()
                        .fold(0, |c_sum: u8, num| c_sum.wrapping_sub(num.wrapping_add(1))),
                    None => return Err(Box::new(RomError::InvalidRom)),
                };
            }
        }
        None => return Err(Box::new(RomError::InvalidRom)),
    };

    return Ok(());
}
