// use std::fs::{self};
// use std::path::Path;

// use crate::emu::RomError;

// cute code bru 😭😭
// pub fn read_rom_header<P: AsRef<Path>>(
//     path: P,
// ) -> Result<(), Box<dyn std::error::Error + 'static>> {
//     // Efficiency (if just gathering header) should go straight to it and not read the whole file
//     match fs::read(path)?.get(0x0100..=0x014F) {
//         Some(s) => {
//             // REFERENCE: https://gbdev.io/pandocs/The_Cartridge_Header.html
//             //
//             // Subtracted from memory access as the original array was cut
//             let start_hex = 0x0100;
//             let cgb_flag_exists;

//             // Gets CGB Flag
//             let Some(cgb_flag) = s.get(0x0143 - start_hex) else {
//                 return Err(Box::new(RomError::CGBMemoryCorrupted));
//             };

//             // Checks if CGB Flag is either option, if not it doesn't exist
//             cgb_flag_exists = cgb_flag == &0x80 || cgb_flag == &0xC0;

//             // Verifies if CGB Flag exists, if not use original title length (16)
//             let title = if let Some(title_slice) = s.get(if cgb_flag_exists {
//                 0x0134 - start_hex..0x0143 - start_hex
//             } else {
//                 0x0134 - start_hex..0x0143 + 1 - start_hex
//             }) {
//                 str::from_utf8(title_slice)?.replace("\0", "")
//             } else {
//                 return Err(Box::new(RomError::TitleMemoryCorrupted));
//             };

//             let checksum_verified;
//             {
//                 let Some(checksum) = s.get(0x014D - start_hex) else {
//                     return Err(Box::new(RomError::ChecksumCorrupted));
//                 };

//                 let checksum_total =
//                     if let Some(checksum_slice) = s.get(0x0134 - start_hex..=0x014C - start_hex) {
//                         checksum_slice
//                             .iter()
//                             .fold(0, |c_sum: u8, num| c_sum.wrapping_sub(num.wrapping_add(1)))
//                     } else {
//                         return Err(Box::new(RomError::HeaderCorrupted));
//                     };

//                 checksum_verified = *checksum == checksum_total;
//             }

//             println!(
//                 "Title: {}\nChecksum: {}",
//                 title,
//                 if checksum_verified {
//                     "VERIFIED"
//                 } else {
//                     "NOT VERIFIED"
//                 }
//             );
//         }
//         None => return Err(Box::new(RomError::InvalidRom)),
//     };

//     return Ok(());
// }
