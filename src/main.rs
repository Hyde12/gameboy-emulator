mod emu;

use minifb::{self, Key, Window, WindowOptions};

use crate::emu::gameboy::Gameboy;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

fn main() {
    let mut gameboy = Gameboy::new();
    // let rom_path = String::from("/home/hyde/Projects/gameboy-emulator/ROMs/dmg-acid2.gb");
    let rom_path = String::from("D:/Projects/GameboyEmulator/ROMs/dmg-acid2.gb");
    gameboy.rom.load_rom(rom_path);

    // TODO: Actual Options Menu
    let mut window_options = WindowOptions::default();
    window_options.scale = minifb::Scale::X4;

    let mut window = Window::new("Gameboy", WIDTH, HEIGHT, window_options).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for _ in 0..17_500 {
            gameboy.tick();
        }

        window
            .update_with_buffer(&gameboy.screen_buffer, WIDTH, HEIGHT)
            .unwrap();
    }
}
