use super::helpers::combine_bytes;
use super::registers::{FlagRegister, Register, Register8};

// // Specifications
// Screen
const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

// Hardware
const RAM_SIZE: usize = 8192;
const VRAM_SIZE: usize = 8192;
const HRAM_SIZE: usize = 127;
const BANK_SIZE: usize = 0xFFFF;
const OAM_SIZE: usize = 40;

// Memory Pointers
const ENTRY_POINT: u16 = 0x0100;

pub struct Gameboy {
    // Hardware
    vram: [u8; VRAM_SIZE],
    wram: [u8; RAM_SIZE],
    oam: [u8; OAM_SIZE],
    memory_map: [u8; BANK_SIZE], // 16 bit address bus

    // Registers
    a_reg: Register,
    b_reg: Register,
    c_reg: Register,
    d_reg: Register,
    e_reg: Register,
    h_reg: Register,
    l_reg: Register,
    f_reg: FlagRegister,
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
            a_reg: Register::new(0x01),
            b_reg: Register::new(0xFF),
            c_reg: Register::new(0x13),
            d_reg: Register::new(0x00),
            e_reg: Register::new(0xC1),
            h_reg: Register::new(0x84),
            l_reg: Register::new(0x03),
            f_reg: FlagRegister::new(0x0),
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

    fn decode(&mut self) {
        let opcode = self.fetch();

        match opcode {
            // NOP
            0x00 => {}

            // LD BC, d16
            0x01 => {
                let high_byte = self.fetch();
                let low_byte = self.fetch();
                let new_val = combine_bytes(high_byte, low_byte);

                self.b_reg.combine_and_write(&mut self.c_reg, new_val);
            }

            // LD B, d8
            0x06 => {
                let new_val = self.fetch();
                self.b_reg.set(new_val)
            }

            // LD C, d8
            0x0E => {
                let new_val = self.fetch();
                self.c_reg.set(new_val)
            }

            // LD D, d8
            0x16 => {
                let new_val = self.fetch();
                self.d_reg.set(new_val)
            }

            // LD E, d8
            0x1E => {
                let new_val = self.fetch();
                self.e_reg.set(new_val)
            }

            // LD H, d8
            0x26 => {
                let new_val = self.fetch();
                self.h_reg.set(new_val)
            }

            // LD L, d8
            0x2E => {
                let new_val = self.fetch();
                self.l_reg.set(new_val)
            }

            0x76 => {
                // halt
                todo!()
            }

            // LD r8, r8 (block 1 pandocs)
            0x40..=0x7F => {
                let dest = (opcode & 0b0011_1000) >> 3;
                let source = opcode & 0b111;
                let source_value = self.read_r8_pattern(source);
                self.write_r8_pattern(dest, source_value);
            }

            // ADD A, r8
            0x80..=0x87 => {
                let operand = self.read_r8_pattern(opcode & 0b111); // wat da r8
                let did_half_carry = ((self.a_reg.read() & 0x0F) + (operand & 0x0F)) > 0x0F; // checks spillover to left nibble
                let (result, overflowed) = self.a_reg.read().overflowing_add(operand); // checks overflow

                // Set registers
                self.a_reg.set(result);
                self.f_reg
                    .flag_reg_set(result == 0, false, did_half_carry, overflowed);
            }

            // ADC A, r8
            0x88..=0x8F => {
                let a = self.a_reg.read();
                let operand = self.read_r8_pattern(opcode & 0b111);
                let carry_flag = self.f_reg.get_carry_flag();

                let (a_r8, did_overflow_1) = a.overflowing_add(operand);
                let (result, did_overflow_2) = a_r8.overflowing_add(carry_flag);

                let did_half_carry = ((a & 0x0F) + (operand & 0x0F) + carry_flag) > 0x0F;

                self.a_reg.set(result);
                self.f_reg.flag_reg_set(
                    result == 0,
                    false,
                    did_half_carry,
                    did_overflow_1 || did_overflow_2,
                );
            }

            // SUB A, r8
            0x90..=0x97 => {
                let a = self.a_reg.read();
                let operand = self.read_r8_pattern(opcode & 0b111);

                let (result, did_underflow) = a.overflowing_sub(operand);
                let did_half_carry = (operand & 0x0F) > (a & 0x0F);

                self.a_reg.set(result);
                self.f_reg
                    .flag_reg_set(result == 0, true, did_half_carry, did_underflow);
            }

            // SBC A, r8
            0x98..=0x9F => {
                let a = self.a_reg.read();
                let operand = self.read_r8_pattern(opcode & 0b111);
                let carry_flag = self.f_reg.get_carry_flag();

                let (a_r8, did_underflow_1) = a.overflowing_sub(operand);
                let (result, did_underflow_2) = a_r8.overflowing_sub(carry_flag);
                let did_half_carry =
                    ((operand & 0x0F) > (a & 0x0F)) || ((carry_flag) > (a_r8 & 0x0F));

                self.a_reg.set(result);
                self.f_reg.flag_reg_set(
                    result == 0,
                    true,
                    did_half_carry,
                    did_underflow_1 || did_underflow_2,
                );
            }

            // JP nn or PC = nn
            0xC3 => {
                let low_byte = self.fetch();
                let high_byte = self.fetch();
                self.pc = combine_bytes(high_byte, low_byte);
            }

            //LD A, n8
            0x3E => {
                let new_val = self.fetch();
                self.a_reg.set(new_val)
            }
            _ => unimplemented!("Operation code not recognized. Code: {}", opcode),
        }
    }

    fn read_r8_pattern(&self, source: u8) -> u8 {
        match source {
            0 => self.b_reg.read(),
            1 => self.c_reg.read(),
            2 => self.d_reg.read(),
            3 => self.e_reg.read(),
            4 => self.h_reg.read(),
            5 => self.l_reg.read(),
            6 => todo!(),
            7 => self.a_reg.read(),
            _ => panic!("R8 Byte matching error. Source: {}", source),
        }
    }

    fn write_r8_pattern(&mut self, destination: u8, value: u8) {
        match destination {
            0 => self.b_reg.set(value),
            1 => self.c_reg.set(value),
            2 => self.d_reg.set(value),
            3 => self.e_reg.set(value),
            4 => self.h_reg.set(value),
            5 => self.l_reg.set(value),
            6 => todo!(),
            7 => self.a_reg.set(value),
            _ => panic!("R8 byte matching error. Destination: {}", destination),
        }
    }
}
