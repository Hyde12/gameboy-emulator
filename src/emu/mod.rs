use std::{error::Error, fmt::Display};

pub mod gameboy;
pub mod helpers;
pub mod registers;
pub mod rom;

#[derive(Debug)]
enum RomError {
    InvalidRom,
    TitleMemoryCorrupted,
    CGBMemoryCorrupted,
    ChecksumCorrupted,
    HeaderCorrupted,
}

impl Display for RomError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error_message_beginning = "Unable to read ROM.";
        match self {
            Self::InvalidRom => println!("{} Invalid Rom.", error_message_beginning),
            Self::TitleMemoryCorrupted => println!(
                "{} Title point in memory corrupted.",
                error_message_beginning
            ),
            Self::CGBMemoryCorrupted => {
                println!("{} CGB point in memory corrupted.", error_message_beginning)
            }

            Self::ChecksumCorrupted => {
                println!(
                    "{} Checksum point in memory corrupted.",
                    error_message_beginning
                )
            }
            Self::HeaderCorrupted => {
                println!("{} Header memory is corrupted.", error_message_beginning)
            }
        }

        Ok(())
    }
}

impl Error for RomError {}
