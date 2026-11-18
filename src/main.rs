mod emu;

use minifb::{self, Key, Window, WindowOptions};

use crate::emu::gameboy::Gameboy;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut gameboy = Gameboy::new();
    let rom_path = String::from("D:/Projects/GameboyEmulator/ROMs/cpu_instrs.gb");
    gameboy.rom.load_rom(rom_path);

    // TODO: Actual Options Menu
    let mut window_options = WindowOptions::default();
    window_options.scale = minifb::Scale::X4;

    let mut window = Window::new("Gameboy", WIDTH, HEIGHT, window_options).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(60);
    print!("dasd");
    while window.is_open() && !window.is_key_down(Key::Escape) {
        gameboy.tick();

        for i in buffer.iter_mut() {
            *i = 0;
        }

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
