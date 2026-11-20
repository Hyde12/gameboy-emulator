use crate::emu::helpers::flag_conditions;

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
        self.pc = self.pc.wrapping_add(1);

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
                    3 => self.inc_sp(),
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
                    3 => self.dec_sp(),
                    _ => unreachable!()
                }
            }

            // ADD HL, r16
            _ if (opcode & 0b1100_1111) == 0b0000_1001 => {
                let r16 = self.read_r16_pattern(opcode >> 4);
                let hl = self.h_reg.combine_and_read(&self.l_reg);

                let did_half_carry = ((r16 & 0x0FFF) + (hl & 0x0FFF)) > 0x0FFF;
                let (result, did_overflow) = r16.overflowing_add(hl);

                self.h_reg.combine_and_write(&mut self.l_reg, result);
                self.f_reg.flag_reg_set(None, false, did_half_carry, did_overflow);
            }

            // INC r8
            _ if (opcode & 0b1100_0111) == 0b0000_0100 => {
                let r8 = opcode >> 3;
                self.inc_8_bit(r8);
            }

            // DEC r8
            _ if (opcode & 0b1100_0111) == 0b0000_0101 => {
                let r8 = opcode >> 3;
                self.dec_8_bit(r8);
            }

            // LD r8, d8
            _ if (opcode & 0b1100_0111) == 0b0000_0110 => {
                let d8 = self.fetch();
                let r8_dest = opcode >> 3;
                self.write_r8_pattern(r8_dest, d8);
            }

            // rlca - cca
            op if (opcode & 0b1100_0111) == 0b0000_0111 => {
                match op >> 3 {
                    0 => self.rlc_8_bit(7, true),
                    1 => self.rrc_8_bit(7, true),
                    2 => self.rl_8_bit(7, true),
                    3 => self.rr_8_bit(7, true),
                    4 => { // daa
                        let result: u8;

                        let a = self.a_reg.read();
                        let h = self.f_reg.get_half_carry_flag();
                        let c = self.f_reg.get_carry_flag();

                        let mut adjustment = 0;
                        if self.f_reg.get_subtraction_flag() == 1 {
                            if h == 1 { adjustment += 0x06 }
                            if c == 1 { adjustment += 0x60 }
                            (result, _, _, _) = self.sub_8_bit(a, adjustment);
                        } else {
                            if h == 1 || (a & 0x0F > 0x09) { adjustment += 0x06 }
                            if c == 1 || (a > 0x99) {
                                adjustment += 0x60;
                                self.f_reg.set_carry_flag(true);
                            } else { self.f_reg.set_carry_flag(false) }
                            (result, _, _, _) = self.add_8_bit(a, adjustment);
                        }

                        self.f_reg.flag_reg_set(result == 0, None, false, None);
                    }
                    5 => { // cpl
                        let flipped_a = !self.a_reg.read();
                        self.a_reg.set(flipped_a);
                        self.f_reg.flag_reg_set(None, true, true, None);
                    }
                    6 => self.f_reg.flag_reg_set(None, false, false, true), // scf
                    7 => { // ccf
                        let flipped_c = !self.f_reg.get_carry_flag();
                        self.f_reg.flag_reg_set(None, false, false, flipped_c == 1);
                    }
                    _ => unreachable!()
                }
            }

            // JR s8
            0x18 => self.jr_s8(),

            // JR cc, s8
            _ if (opcode & 0b1110_0111) == 0b0010_0000 => {
                let cond = (opcode & 0b0001_1000) >> 3;

                if self.check_flag_cond(cond) {
                    self.jr_s8();
                }
            }

            // STOP
            0x10 => {
                // should prob js implement halt code
                // todo!()
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

            // 8-bit arithmetic (block 2 & 3.1 opcode)
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

            // RET cond
            op if (opcode & 0b1110_0111) == 0b1100_0000 => {
                let cond = (op >> 3) & 0b0000_0011;

                if self.check_flag_cond(cond) {
                    self.return_from_subroutine();
                }
            }

            // RET
            0xC9 => self.return_from_subroutine(),

            // RETI
            0xD9 => self.return_from_subroutine(), // todo, enable interrupt

            // JP cond, d16
            op if (opcode & 0b1110_0111) == 0b1100_0010 => {
                let d16 = self.fetch_d16();
                let cond = (op >> 3) & 0b0000_0011;

                if self.check_flag_cond(cond) {
                    self.pc = d16;
                }
            }

            // JP a16
            0xC3 => self.pc = self.fetch_d16(),

            // JP HL
            0xE9 => self.pc = self.h_reg.combine_and_read(&self.l_reg),

            // CALL COND, a16
            op if (opcode & 0b1110_0111) == 0b1100_0100 => {
                let cond = (op >> 3) & 0b0000_0011;
                let call_instr = self.fetch_d16();

                if self.check_flag_cond(cond) {
                    self.call_a16(call_instr);
                }
            }

            // CALL a16
            0xCD => {
                let call_instr = self.fetch_d16();
                self.call_a16(call_instr);
            }

            // RST tgt3
            op if (opcode & 0b1100_0111) == 0b1100_0111 => {
                let tgt3 = (op >> 3) & 0b0000_0111;

                self.push_stack((self.pc >> 8) as u8);
                self.push_stack((self.pc & 0x00FF) as u8);

                self.pc = (tgt3 as u16) * 8;
            }

            // POP r16stk
            op if (opcode & 0b1100_1111) == 0b1100_0001 => {
                let stack_1 = self.pop_stack();
                let stack_2 = self.pop_stack();
                let d16 = combine_bytes(stack_2, stack_1);

                match (op & 0b0000_0011) >> 4 {
                    0 => self.b_reg.combine_and_write(&mut self.c_reg, d16),
                    1 => self.d_reg.combine_and_write(&mut self.e_reg, d16),
                    2 => self.h_reg.combine_and_write(&mut self.l_reg, d16),
                    3 => self.a_reg.combine_and_write(&mut self.f_reg, d16),
                    _ => unreachable!()
                }
            }

            // PUSH r16stk
            op if (opcode & 0b1100_1111) == 0b1100_0101 => {
                match (op & 0b0000_0011) >> 4 {
                    0 => {
                        self.push_stack(self.b_reg.read());
                        self.push_stack(self.c_reg.read());
                    },
                    1 => {
                        self.push_stack(self.d_reg.read());
                        self.push_stack(self.e_reg.read());
                    },
                    2 => {
                        self.push_stack(self.h_reg.read());
                        self.push_stack(self.l_reg.read());
                    },
                    3 => {
                        self.push_stack(self.a_reg.read());
                        self.push_stack(self.f_reg.read());
                    },
                    _ => unreachable!()
                }
            }
            _ => unimplemented!("Operation code not recognized. Code: {}", opcode),
        }
    }
}

// PATTERN MATCHING
impl Gameboy {
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

    fn check_flag_cond(&self, cond: u8) -> bool {
        return match cond {
            0 => { self.f_reg.get_zero_flag() == 0 }
            1 => { self.f_reg.get_zero_flag() == 1 }
            2 => { self.f_reg.get_carry_flag() == 0 }
            3 => { self.f_reg.get_carry_flag() == 1 }
            _ => unreachable!(),
        };
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

// ARITHMETIC
impl Gameboy {
    fn add_8_bit(&self, r8: u8, operand: u8) -> (u8, bool, bool, bool) {
        let (result, did_carry) = r8.overflowing_add(operand);
        let did_half_carry = ((r8 & 0x0F) + (operand & 0x0F)) > 0x0F;

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

    fn inc_8_bit(&mut self, r8: u8) {
        let (result, sub_flag, did_half_carry, _) = self.add_8_bit(self.read_r8_pattern(r8), 1);
        self.write_r8_pattern(r8, result);
        self.f_reg.flag_reg_set(result == 0, sub_flag, did_half_carry, None);
    }

    fn inc_sp(&mut self) {
        self.sp = self.sp.wrapping_add(1);
    }

    fn dec_8_bit(&mut self, r8: u8) {
        let (result, sub_flag, did_half_carry, _) = self.sub_8_bit(self.read_r8_pattern(r8), 1);
        self.write_r8_pattern(r8, result);
        self.f_reg.flag_reg_set(result == 0, sub_flag, did_half_carry, None);
    }

    fn dec_sp(&mut self) {
        self.sp = self.sp.wrapping_sub(1)
    }
}

// MEMORY MAGIC
impl Gameboy {
    fn rlc_8_bit(&mut self, r8: u8, rlca: bool) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = (val & 0b1000_0000) >> 7;
        let result = (val << 1) | shifted_bit;

        self.f_reg.flag_reg_set(!rlca && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    fn rrc_8_bit(&mut self, r8: u8, rrca: bool) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = val & 0b0000_0001;
        let result = (val >> 1) | (shifted_bit << 7);

        self.f_reg.flag_reg_set(!rrca && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    fn rl_8_bit(&mut self, r8: u8, rla: bool) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = (val & 0b1000_0000) >> 7;
        let result = (val << 1) | self.f_reg.get_carry_flag();

        self.f_reg.flag_reg_set(!rla && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    fn rr_8_bit(&mut self, r8: u8, rra: bool) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = val & 0b0000_0001;
        let result = (val >> 1) | (self.f_reg.get_carry_flag() << 7);

        self.f_reg.flag_reg_set(!rra && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    fn jr_s8(&mut self) {
        let s8 = self.fetch() as i8;
        self.pc = self.pc.wrapping_add_signed(s8 as i16)
    }

    fn return_from_subroutine(&mut self) {
        let return_addr_low = self.pop_stack();
        let return_addr_high = self.pop_stack();

        let return_addr = combine_bytes(return_addr_high, return_addr_low);
        self.pc = return_addr;
    }

    fn call_a16(&mut self, call_instr: u16) {
        self.push_stack((self.pc >> 4) as u8);
        self.push_stack((self.pc & 0b0000_1111) as u8);
        self.pc = call_instr;
    }

    fn pop_stack(&mut self) -> u8 {
        let val = self.read_bus(self.sp as usize);
        self.inc_sp();
        val
    }

    fn push_stack(&mut self, value: u8) {
        self.dec_sp();
        self.write_bus(self.sp as usize, value);
    }
}
