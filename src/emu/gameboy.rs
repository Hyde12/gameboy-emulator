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

            // 8-bit arithmetic (block 2 opcode)
            0x80..=0xBF | _ if (opcode & 0b1100_0110) == 0b1100_0110 => {
                let arithmetic = (opcode & 0b0011_1000) >> 3;

                let a = self.a_reg.read();
                let operand;

                if opcode & 0b0100_0000 > 0 { // check if value is immediate or not
                    operand = self.fetch();
                } else {
                    operand = self.read_r8_pattern(opcode & 0b111);
                }

                // Z N H C
                let flags: (u8, bool, bool, bool);

                match arithmetic {
                    0b000 => { // ADD A, r8
                        flags = self.add_8_bit(a, operand);
                    }
                    0b001 => { // ADC A, r8
                        flags = self.adc_8_bit(a, operand);
                    }
                    0b010 => { // SUB A, r8
                        flags = self.sub_8_bit(a, operand);
                    }
                    0b011 => { // SBC A, r8
                        flags = self.sbc_8_bit(a, operand)
                    }
                    0b100 => { // AND A, r8
                        flags = self.and_8_bit(a, operand)
                    }
                    0b101 => { // XOR A, r8
                        flags = self.xor_8_bit(a, operand)
                    }
                    0b110 => { // OR A, r8
                        flags = self.or_8_bit(a, operand)
                    }
                    0b111 => { // CP A, r8
                        flags = self.sub_8_bit(a, operand)
                    }
                    _ => unreachable!(),
                }

                if arithmetic != 0b111 {
                    self.a_reg.set(flags.0);
                }

                self.f_reg
                    .flag_reg_set(flags.0 == 0, flags.1, flags.2, flags.3);
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


}

impl Gameboy {
    fn add_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        let (result, did_carry) = a.overflowing_add(operand);
        let did_half_carry = ((a & 0x0F) + (operand & 0x0F)) > 0x0F;

        (result, false, did_half_carry, did_carry)
    }

    fn adc_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        let carry_flag = self.f_reg.get_carry_flag();

        let (a_r8, did_overflow_1) = a.overflowing_add(operand);
        let (a_r8_c, did_overflow_2) = a_r8.overflowing_add(carry_flag);

        let did_half_carry = ((a & 0x0F) + (operand & 0x0F) + carry_flag) > 0x0F;
        let did_carry = did_overflow_1 || did_overflow_2;

        (a_r8_c, false, did_half_carry, did_carry)
    }

    fn sub_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        let (result, did_carry) = a.overflowing_sub(operand);
        let did_half_carry = (operand & 0x0F) > (a & 0x0F);

        (result, true, did_half_carry, did_carry)
    }

    fn sbc_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        let carry_flag = self.f_reg.get_carry_flag();

        let (a_r8, did_underflow_1) = a.overflowing_sub(operand);
        let (a_r8_c, did_underflow_2) = a_r8.overflowing_sub(carry_flag);

        let did_half_carry = (operand & 0x0F) + 1 > a & 0x0F;
        let did_carry = did_underflow_1 || did_underflow_2;

        (a_r8_c, true, did_half_carry, did_carry)
    }

    fn and_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        (a & operand, false, true, false)
    }

    fn xor_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        (a ^ operand, false, false, false)
    }

    fn or_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        (a | operand, false, false, false)
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
