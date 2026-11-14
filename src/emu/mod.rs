use std::{error::Error, fmt::Display};

use crate::emu::RomError::InvalidRom;

pub mod rom_reader;

#[derive(Debug)]
enum RomError {
    InvalidRom,
}

impl Display for RomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidRom => println!("Invalid Rom."),
        }

        Ok(())
    }
}

impl Error for RomError {}
