use super::helpers::{combine_bytes};
use super::rom::ROMData;
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
const ENTRY_POINT: usize = 0x0100;
const ROM_BANK_0: usize = 0x0000;
const ROM_BANK_1: usize = 0x4000;
const VRAM_START: usize = 0x8000;
const EXT_RAM_START: usize = 0xA000;
const WRAM_1_START: usize = 0xC000;
const WRAM_2_START: usize = 0xD000;
const ECH_RAM_START: usize = 0xE000;
const OAM_START: usize = 0xFE00;
const UNUSABLE_START: usize = 0xFEA0;
const IO_REGISTERS_START: usize = 0xFF00;
const H_RAM_START: usize = 0xFF80;
const IE_REGISTER: usize = 0xFFFF;

pub struct Gameboy {
    // Hardware
    pub rom: ROMData,
    vram: [u8; VRAM_SIZE],
    wram_1: [u8; RAM_SIZE / 2],
    wram_2: [u8; RAM_SIZE / 2],
    oam: [u8; OAM_SIZE],
    hram: [u8; HRAM_SIZE],

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
    pub pc: u16,
}

impl Gameboy {
    pub fn new() -> Self {
        Gameboy {
            rom: ROMData::new(),
            vram: [0; VRAM_SIZE],
            wram_1: [0; RAM_SIZE / 2],
            wram_2: [0; RAM_SIZE / 2],
            oam: [0; OAM_SIZE],
            hram: [0; HRAM_SIZE],
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

    pub fn tick(&mut self) {
        let opcode = self.fetch();
        println!("{}", opcode);
        self.decode(opcode);
    }

    fn fetch(&mut self) -> u8 {
        let byte = self.read_bus(self.pc as usize);
        self.pc += 1;

        byte
    }

    fn fetch_d16(&mut self) -> u16 {
        let low_byte = self.fetch();
        let high_byte = self.fetch();

        combine_bytes(high_byte, low_byte)
    }

    fn decode(&mut self, opcode: u8) {
        match opcode {
            // NOP
            0x00 => {}

            // LD r16, d16
            _ if (opcode & 0b1100_1111) == 0b0000_0001 => {
                let destination = (opcode & 0b0011_0000) >> 4;
                let d16 = self.fetch_d16();

                self.write_r16_pattern(destination, d16);
            }

            // LD (r16), A
            _ if (opcode & 0b1100_1111) == 0b0000_0010 => {
                let destination = (opcode & 0b0011_0000) >> 4;
                let value = self.a_reg.read();

                self.write_r16mem_pattern(destination, value);
            }

            // LD A, r16
            _ if (opcode & 0b1100_1111) == 0b0000_1010 => {
                let source = (opcode & 0b0011_0000) >> 4;
                let value = self.read_r16mem_pattern(source);
                self.a_reg.set(value);
            }

            // LD (a16), SP
            0x08 => {
                let a16 = self.fetch_d16();

                self.write_bus(a16 as usize, (self.sp & 0x00FF) as u8);
                self.write_bus(a16.wrapping_add(1) as usize, ((self.sp & 0xFF00) >> 8) as u8);
            }

            // INC r16
            _ if (opcode & 0b1100_1111) == 0b0000_0011 => {
                let r16 = opcode >> 4;

                match r16 {
                    0 => self.b_reg.combine_and_increment(&mut self.c_reg),
                    1 => self.d_reg.combine_and_increment(&mut self.e_reg),
                    2 => self.h_reg.combine_and_increment(&mut self.l_reg),
                    3 => self.sp = self.sp.wrapping_add(1),
                    _ => unreachable!()
                }
            }

            // DEC 16
            _ if (opcode & 0b1100_1111) == 0b0000_1011 => {
                let r16 = opcode >> 4;

                match r16 {
                    0 => self.b_reg.combine_and_decrement(&mut self.c_reg),
                    1 => self.d_reg.combine_and_decrement(&mut self.e_reg),
                    2 => self.h_reg.combine_and_decrement(&mut self.l_reg),
                    3 => self.sp = self.sp.wrapping_sub(1),
                    _ => unreachable!()
                }
            }

            // ADD HL, r16
            _ if (opcode & 0b1100_1111) == 0b0000_1001 => {
                let r16 = self.read_r16_pattern(opcode >> 4);
                let hl = self.h_reg.combine_and_read(&self.l_reg);

                let did_half_carry = ((r16 & 0x00FF) + (hl & 0x00FF)) > 0x00FF;
                let (result, did_overflow) = r16.overflowing_add(hl);

                self.h_reg.combine_and_write(&mut self.l_reg, result);
                self.f_reg.flag_reg_set(None, false, did_half_carry, did_overflow);
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
                self.pc = self.fetch_d16();
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
            6 => self.read_bus(self.h_reg.combine_and_read(&self.l_reg) as usize),
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
            6 => self.write_bus(self.h_reg.combine_and_read(&self.l_reg) as usize, value),
            7 => self.a_reg.set(value),
            _ => unreachable!("R8 byte matching error. Destination: {}", destination),
        }
    }

    fn read_r16_pattern(&mut self, source: u8) -> u16 {
        match source {
            0 => self.b_reg.combine_and_read(&self.c_reg),
            1 => self.d_reg.combine_and_read(&self.e_reg),
            2 => self.h_reg.combine_and_read(&self.l_reg),
            3 => self.sp,
            _ => unreachable!("R16 byte matching error. Source: {}", source),
        }
    }

    fn write_r16_pattern(&mut self, destination: u8, value: u16) {
        match destination {
            0 => self.b_reg.combine_and_write(&mut self.c_reg, value),
            1 => self.d_reg.combine_and_write(&mut self.e_reg, value),
            2 => self.h_reg.combine_and_write(&mut self.l_reg, value),
            3 => self.sp = value,
            _ => unreachable!("R16 byte matching error. Destination: {}", destination),
        }
    }

    fn read_r16mem_pattern(&mut self, source: u8) -> u8  {
        match source {
            0 => self.read_bus(self.b_reg.combine_and_read(&self.c_reg) as usize),
            1 => self.read_bus(self.d_reg.combine_and_read(&self.e_reg) as usize),
            2 => {
                let val = self.read_bus(self.h_reg.combine_and_read(&self.l_reg) as usize);
                self.h_reg.combine_and_increment(&mut self.l_reg);
                val
            },
            3 => {
                let val = self.read_bus(self.h_reg.combine_and_read(&self.l_reg) as usize);
                self.h_reg.combine_and_decrement(&mut self.l_reg);
                val
            },
            _ => unreachable!("R16_MEMORY byte matching error. Source: {}", source),
        }
    }

    fn write_r16mem_pattern(&mut self, destination: u8, value: u8) {
        match destination {
            0 => self.write_bus(self.b_reg.combine_and_read(&self.c_reg) as usize, value),
            1 => self.write_bus(self.d_reg.combine_and_read(&self.e_reg) as usize, value),
            2 => {
                self.write_bus(self.h_reg.combine_and_read(&self.l_reg) as usize, value);
                self.h_reg.combine_and_increment(&mut self.l_reg);
            },
            3 => {
                self.write_bus(self.h_reg.combine_and_read(&self.l_reg) as usize, value);
                self.h_reg.combine_and_decrement(&mut self.l_reg);
            },
            _ => unreachable!("R16_MEMORY byte matching error. Destination: {}", destination),
        }
    }

    fn read_bus(&self, address: usize) -> u8 {
        match address {
            ROM_BANK_0..ROM_BANK_1 => self.rom.data[address - ROM_BANK_0],
            ROM_BANK_1..VRAM_START => self.rom.data[address - ROM_BANK_0],
            VRAM_START..EXT_RAM_START => self.vram[address - VRAM_START],
            EXT_RAM_START..WRAM_1_START => self.rom.external_ram[address - EXT_RAM_START],
            WRAM_1_START..WRAM_2_START => self.wram_1[address - WRAM_1_START],
            WRAM_2_START..ECH_RAM_START => self.wram_2[address - WRAM_2_START],
            ECH_RAM_START..OAM_START => todo!(), // mirror of wram 1
            OAM_START..UNUSABLE_START => self.oam[address - OAM_START],
            IO_REGISTERS_START..H_RAM_START => todo!(),
            H_RAM_START..IE_REGISTER => self.hram[address - H_RAM_START],
            _ => unreachable!()
        }
    }

    fn write_bus(&mut self, address: usize, value: u8) {
        match address {
            VRAM_START..EXT_RAM_START => self.vram[address - VRAM_START] = value,
            EXT_RAM_START..WRAM_1_START => self.rom.external_ram[address - EXT_RAM_START] = value,
            WRAM_1_START..WRAM_2_START => self.wram_1[address - WRAM_1_START] = value,
            WRAM_2_START..ECH_RAM_START => self.wram_2[address - WRAM_2_START] = value,
            ECH_RAM_START..OAM_START => todo!(), // mirror of wram 1
            OAM_START..UNUSABLE_START => self.oam[address - OAM_START] = value,
            IO_REGISTERS_START..H_RAM_START => todo!(),
            H_RAM_START..IE_REGISTER => self.hram[address - H_RAM_START] = value,
            _ => unreachable!()
        }
    }
}
