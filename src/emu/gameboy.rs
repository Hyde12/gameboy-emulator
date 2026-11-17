const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

const RAM_SIZE: usize = 8192;
const VRAM_SIZE: usize = 8192;
const HRAM_SIZE: usize = 127;
const BANK_SIZE: usize = 0xFFFF;
const OAM_SIZE: usize = 40;

const ENTRY_POINT: u16 = 0x0100;

pub struct Gameboy {
    // Hardware
    vram: [u8; VRAM_SIZE],
    wram: [u8; RAM_SIZE],
    oam: [u8; OAM_SIZE],
    memory_map: [u8; BANK_SIZE], // 16 bit address bus

    // Registers
    a_reg: u8,
    f_reg: FlagRegister,
    b_reg: u8,
    c_reg: u8,
    d_reg: u8,
    e_reg: u8,
    h_reg: u8,
    l_reg: u8,
    sp: u16,
    pc: u16,
}

impl Gameboy {
    pub fn new() -> Self {
        Gameboy {
            vram: [0; VRAM_SIZE],
            wram: [0; VRAM_SIZE],
            oam: [0; OAM_SIZE],
            memory_map: [0; BANK_SIZE],
            a_reg: 0x01,
            f_reg: FlagRegister { f_reg: 0x0 },
            b_reg: 0xFF,
            c_reg: 0x13,
            d_reg: 0x00,
            e_reg: 0xC1,
            h_reg: 0x84,
            l_reg: 0x03,
            pc: ENTRY_POINT as u16,
            sp: 0xFFFE,
        }
    }

    pub fn load_rom(&mut self, rom_data: &[u8]) {
        self.memory_map[0x0000..=0x7FFF].copy_from_slice(rom_data); // loads ROM data onto memory map
    }

    fn fetch(&mut self) -> u8 {
        let byte = self.memory_map[self.pc as usize];
        self.pc += 1;

        byte
    }
}

struct FlagRegister {
    f_reg: u8,
}

impl FlagRegister {
    fn zero_flag(&self) -> bool {
        (self.f_reg & 0b0100_0000) >> 7 == 1
    }

    fn subtraction_flag(&self) -> bool {
        (self.f_reg & 0b1000_0000) >> 7 == 1
    }

    fn half_carry_flag(&self) -> bool {
        (self.f_reg & 0b0001_0000) >> 7 == 1
    }

    fn carry_flag(&self) -> bool {
        (self.f_reg & 0b0000_1000) >> 7 == 1
    }
}
