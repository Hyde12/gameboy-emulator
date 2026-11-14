mod emu;

use crate::emu::rom_reader::read_rom_header;

use minifb::{self, Key, Window, WindowOptions};
use std::path::Path;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let rom_path = Path::new("D:/Projects/GameboyEmulator/Roms/cpu_instrs.gb");

    // TODO: Actual Options Menu
    let mut window_options = WindowOptions::default();
    window_options.scale = minifb::Scale::X4;
    //

    let mut window = Window::new("Gameboy", WIDTH, HEIGHT, window_options).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(60);
    read_rom_header(rom_path);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for i in buffer.iter_mut() {
            *i = 0; // write something more funny here!
        }

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
