use std::io::{self, Write};

use crate::emu::gameboy::PPUMode::{Mode0, Mode3};
use crate::emu::helpers::{pallete_conversion, tile_data_reading};

use super::helpers::combine_bytes;
use super::registers::{FlagRegister, Register, Register8};
use super::rom::ROMData;

// // // Specifications
// // Screen
const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

// // Hardware
const RAM_SIZE: usize = 8192;
const VRAM_SIZE: usize = 8192;
const HRAM_SIZE: usize = 127;
const OAM_SIZE: usize = 160;

// // Memory Pointers
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

// // Hardware Registers
// Timers
const DIV: usize = 0xFF04;
const TIMA: usize = 0xFF05;
const TMA: usize = 0xFF06;
const TAC: usize = 0xFF07;

// Interrupts
const INTERRUPT_FLAG: usize = 0xFF0F;
const INTERRUPT_ENABLE: usize = 0xFFFF;

// Graphics
const LCDC: usize = 0xFF40;
const STAT: usize = 0xFF41;
const SCY: usize = 0xFF42;
const SCX: usize = 0xFF43;
const LY: usize = 0xFF44;
const LYC: usize = 0xFF45;
const WY: usize = 0xFF4A;
const WX: usize = 0xFF4B;

#[derive(PartialEq)]
enum PPUMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
}

struct PixelFifo {
    pixel_fifo: [u8; 16],
    fetched_tile_data: u16,
    lx: u8,
    len: u8,
    discarded_pixels: u8,
}

struct PPU {
    mode: PPUMode,
    dots: u16,
    y_condition: bool,

    // mode 3
    fifo: PixelFifo,
    fetcher_x: u8,
}

pub struct Gameboy {
    // // Hardware
    pub rom: ROMData,
    pub screen_buffer: [u32; SCREEN_WIDTH * SCREEN_HEIGHT],
    ppu: PPU,
    vram: [u8; VRAM_SIZE],
    wram_1: [u8; RAM_SIZE / 2],
    wram_2: [u8; RAM_SIZE / 2],
    oam: [u8; OAM_SIZE],
    hram: [u8; HRAM_SIZE],

    // // Registers
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

    // // Hardware Registers
    // Timers
    sysclck: u16,
    io_registers: [u8; 128], // temporary
    tima: u8,
    tma: u8,
    tac: u8,

    // Interrupts
    ime: u8,
    interrupt_enable: u8,
    interrupt_flag: u8,
    stat: u8,

    // Graphics
    lcdc: u8,
    ly: u8,
    lyc: u8,
    scy: u8,
    scx: u8,
    wy: u8,
    wx: u8,

    // // Other Flags
    ei_pending: bool,
    is_halted: bool,
    halt_bug: bool,
}

impl Gameboy {
    pub fn new() -> Self {
        Gameboy {
            // hardware
            screen_buffer: [0x00; SCREEN_WIDTH * SCREEN_HEIGHT],
            ppu: PPU {
                mode: PPUMode::Mode2,
                y_condition: false,
                fifo: PixelFifo {
                    pixel_fifo: [0; 16],
                    fetched_tile_data: 0,
                    lx: 0,
                    len: 0,
                    discarded_pixels: 0,
                },
                fetcher_x: 0,
                dots: 0,
            },
            rom: ROMData::new(),
            vram: [0; VRAM_SIZE],
            wram_1: [0; RAM_SIZE / 2],
            wram_2: [0; RAM_SIZE / 2],
            oam: [0; OAM_SIZE],
            hram: [0; HRAM_SIZE],

            // registers
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

            // interrupts
            ime: 0x00,
            interrupt_enable: 0x00,
            interrupt_flag: 0x00,
            io_registers: [0; 128],

            // timing
            sysclck: 0,
            tima: 0x00,
            tma: 0x00,
            tac: 0x00,
            stat: 0x81,

            // graphics
            lcdc: 0x91,
            ly: 0x00,
            lyc: 0x00,
            scy: 0x00,
            scx: 0x00,
            wy: 0x00,
            wx: 0x00,

            // other flags
            is_halted: false,
            halt_bug: false,
            ei_pending: false,
        }
    }

    pub fn tick(&mut self) {
        if self.has_pending_interrupt() {
            let was_halted = self.is_halted;
            self.is_halted = false;

            if self.ime == 1 {
                if was_halted {
                    self.t_cycle_add(4);
                }
                self.call_interrupt_handler();
                return;
            }
        }

        if !self.is_halted {
            let opcode = self.fetch();

            let was_pending = self.ei_pending;

            if self.halt_bug {
                self.halt_bug = false;
                self.pc = self.pc.wrapping_sub(1);
            }

            self.decode(opcode);

            if was_pending && self.ei_pending {
                self.ime = 1;
                self.ei_pending = false
            }
        } else {
            self.t_cycle_add(4);
        }
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
                self.write_bus(
                    a16.wrapping_add(1) as usize,
                    ((self.sp & 0xFF00) >> 8) as u8,
                );
            }

            // INC r16
            _ if (opcode & 0b1100_1111) == 0b0000_0011 => {
                let r16 = opcode >> 4;
                self.t_cycle_add(4);

                match r16 {
                    0 => self.b_reg.combine_and_increment(&mut self.c_reg),
                    1 => self.d_reg.combine_and_increment(&mut self.e_reg),
                    2 => self.h_reg.combine_and_increment(&mut self.l_reg),
                    3 => self.inc_sp(),
                    _ => unreachable!(),
                }
            }

            // DEC 16
            _ if (opcode & 0b1100_1111) == 0b0000_1011 => {
                let r16 = opcode >> 4;
                self.t_cycle_add(4);

                match r16 {
                    0 => self.b_reg.combine_and_decrement(&mut self.c_reg),
                    1 => self.d_reg.combine_and_decrement(&mut self.e_reg),
                    2 => self.h_reg.combine_and_decrement(&mut self.l_reg),
                    3 => self.dec_sp(),
                    _ => unreachable!("Invalid R16 Pattern. Pattern: {}", r16),
                }
            }

            // ADD HL, r16
            _ if (opcode & 0b1100_1111) == 0b0000_1001 => {
                self.t_cycle_add(4);

                let r16 = self.read_r16_pattern(opcode >> 4);
                let hl = self.h_reg.combine_and_read(&self.l_reg);

                let did_half_carry = ((r16 & 0x0FFF) + (hl & 0x0FFF)) > 0x0FFF;
                let (result, did_overflow) = r16.overflowing_add(hl);

                self.h_reg.combine_and_write(&mut self.l_reg, result);
                self.f_reg
                    .flag_reg_set(None, false, did_half_carry, did_overflow);
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
                    4 => {
                        // daa
                        let result: u8;

                        let a = self.a_reg.read();
                        let h = self.f_reg.get_half_carry_flag();
                        let c = self.f_reg.get_carry_flag();

                        let mut adjustment = 0;

                        if self.f_reg.get_subtraction_flag() == 1 {
                            if h == 1 {
                                adjustment += 0x06
                            }
                            if c == 1 {
                                adjustment += 0x60
                            }
                            (result, _, _, _) = self.sub_8_bit(a, adjustment);
                        } else {
                            if h == 1 || (a & 0x0F > 0x09) {
                                adjustment += 0x06
                            }
                            if c == 1 || (a > 0x99) {
                                adjustment += 0x60;
                                self.f_reg.set_carry_flag(true);
                            } else {
                                self.f_reg.set_carry_flag(false)
                            }
                            (result, _, _, _) = self.add_8_bit(a, adjustment);
                        }

                        self.a_reg.set(result);
                        self.f_reg.flag_reg_set(result == 0, None, false, None);
                    }
                    5 => {
                        // cpl
                        let flipped_a = !self.a_reg.read();
                        self.a_reg.set(flipped_a);
                        self.f_reg.flag_reg_set(None, true, true, None);
                    }
                    6 => self.f_reg.flag_reg_set(None, false, false, true), // scf
                    7 => {
                        // ccf
                        self.f_reg.flag_reg_set(
                            None,
                            false,
                            false,
                            self.f_reg.get_carry_flag() == 0,
                        );
                    }
                    _ => unreachable!("wtf"),
                }
            }

            // JR s8
            0x18 => {
                self.t_cycle_add(4);

                let s8 = self.fetch() as i8;
                self.jr_s8(s8);
            }

            // JR cc, s8
            _ if (opcode & 0b1110_0111) == 0b0010_0000 => {
                let s8 = self.fetch() as i8;
                let cond = (opcode & 0b0001_1000) >> 3;

                if self.check_flag_cond(cond) {
                    self.t_cycle_add(4);
                    self.jr_s8(s8);
                }
            }

            // STOP & HALT
            0x76 => {
                if self.ime == 0 && self.has_pending_interrupt() {
                    self.halt_bug = true;
                } else {
                    self.is_halted = true;
                }
            }

            // LD r8, r8 (block 1 pandocs)
            0x40..=0x7F => {
                let dest = (opcode & 0b0011_1000) >> 3;
                let source = opcode & 0b111;
                let source_value = self.read_r8_pattern(source);
                self.write_r8_pattern(dest, source_value);
            }

            // 8-bit arithmetic (block 2 & 3.1 opcode)
            _ if (0x80..=0xBF).contains(&opcode) || (opcode & 0b1100_0111) == 0b1100_0110 => {
                let arithmetic = (opcode & 0b0011_1000) >> 3;

                let a = self.a_reg.read();
                let operand;

                if opcode & 0b0100_0000 > 0 {
                    // check if value is immediate or not
                    operand = self.fetch();
                } else {
                    operand = self.read_r8_pattern(opcode & 0b111);
                }

                // Z N H C
                let flags: (u8, bool, bool, bool);

                match arithmetic {
                    0b000 => {
                        // ADD A, r8
                        flags = self.add_8_bit(a, operand);
                    }
                    0b001 => {
                        // ADC A, r8
                        flags = self.adc_8_bit(a, operand);
                    }
                    0b010 => {
                        // SUB A, r8
                        flags = self.sub_8_bit(a, operand);
                    }
                    0b011 => {
                        // SBC A, r8
                        flags = self.sbc_8_bit(a, operand)
                    }
                    0b100 => {
                        // AND A, r8
                        flags = self.and_8_bit(a, operand)
                    }
                    0b101 => {
                        // XOR A, r8
                        flags = self.xor_8_bit(a, operand)
                    }
                    0b110 => {
                        // OR A, r8
                        flags = self.or_8_bit(a, operand)
                    }
                    0b111 => {
                        // CP A, r8
                        flags = self.sub_8_bit(a, operand)
                    }
                    _ => unreachable!("wtf2"),
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
                self.t_cycle_add(4);

                if self.check_flag_cond(cond) {
                    self.return_from_subroutine();
                }
            }

            // RET
            0xC9 => self.return_from_subroutine(),

            // RETI
            0xD9 => {
                self.ime = 1;
                self.return_from_subroutine()
            }

            // JP cond, d16
            op if (opcode & 0b1110_0111) == 0b1100_0010 => {
                let d16 = self.fetch_d16();
                let cond = (op >> 3) & 0b0000_0011;

                if self.check_flag_cond(cond) {
                    self.t_cycle_add(4);
                    self.pc = d16;
                }
            }

            // JP a16
            0xC3 => {
                self.t_cycle_add(4);
                self.pc = self.fetch_d16()
            }

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

                match (op & 0b0011_0000) >> 4 {
                    0 => self.b_reg.combine_and_write(&mut self.c_reg, d16),
                    1 => self.d_reg.combine_and_write(&mut self.e_reg, d16),
                    2 => self.h_reg.combine_and_write(&mut self.l_reg, d16),
                    3 => self.a_reg.combine_and_write(&mut self.f_reg, d16 & 0xFFF0),
                    _ => unreachable!(
                        "Not a valid r16stk pattern. Pattern: {}",
                        (op & 0b0000_0011) >> 4
                    ),
                }
            }

            // PUSH r16stk
            op if (opcode & 0b1100_1111) == 0b1100_0101 => match (op & 0b0011_0000) >> 4 {
                0 => {
                    self.push_stack(self.b_reg.read());
                    self.push_stack(self.c_reg.read());
                }
                1 => {
                    self.push_stack(self.d_reg.read());
                    self.push_stack(self.e_reg.read());
                }
                2 => {
                    self.push_stack(self.h_reg.read());
                    self.push_stack(self.l_reg.read());
                }
                3 => {
                    self.push_stack(self.a_reg.read());
                    self.push_stack(self.f_reg.read());
                }
                _ => unreachable!(
                    "Not a valid r16stk pattern. Pattern: {}",
                    (op & 0b0000_0011) >> 4
                ),
            },

            // PREFIX
            0xCB => {
                let op = self.fetch();
                let r8 = op & 0b0000_0111;
                let b3 = (op & 0b0011_1000) >> 3;

                match op >> 3 {
                    0 => self.rlc_8_bit(r8, false),
                    1 => self.rrc_8_bit(r8, false),
                    2 => self.rl_8_bit(r8, false),
                    3 => self.rr_8_bit(r8, false),
                    4 => self.sla_8_bit(r8),
                    5 => self.sra_8_bit(r8),
                    6 => self.swap(r8),
                    7 => self.srl_8_bit(r8),
                    _ => {
                        let r8_value = self.read_r8_pattern(r8);

                        match op >> 6 {
                            1 => {
                                self.f_reg.flag_reg_set(
                                    (r8_value >> b3) & 1 != 1,
                                    false,
                                    true,
                                    None,
                                );
                            }
                            2 => {
                                self.write_r8_pattern(r8, r8_value & !(1 << b3));
                            }
                            3 => {
                                self.write_r8_pattern(r8, r8_value | 1 << b3);
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }

            // LD A, [d8] or LD [d8], A
            0xE0 | 0xF0 => {
                let d8 = self.fetch() as usize;
                let mem_addr = (0xFF00 + d8) as usize;

                if opcode == 0xE0 {
                    self.write_bus(mem_addr, self.a_reg.read());
                } else {
                    let memory_val = self.read_bus(mem_addr);
                    self.a_reg.set(memory_val);
                }
            }

            // LD [0xFF00 + C], A or LD A, [0xFF00 + C]
            0xE2 | 0xF2 => {
                let c = 0xFF00 + (self.c_reg.read() as usize);

                if opcode == 0xE2 {
                    self.write_bus(c as usize, self.a_reg.read());
                } else {
                    let memory_val = self.read_bus(c as usize);
                    self.a_reg.set(memory_val);
                }
            }

            // LD (a16), A or LD A, (a16)
            0xEA | 0xFA => {
                let d16 = self.fetch_d16() as usize;

                if opcode == 0xEA {
                    self.write_bus(d16, self.a_reg.read());
                } else {
                    let memory_val = self.read_bus(d16);
                    self.a_reg.set(memory_val);
                }
            }

            // ADD SP, s8 or LD HL, SP+s8
            0xE8 | 0xF8 => {
                self.t_cycle_add(4);

                let d8 = self.fetch();
                let s8 = d8 as i8;

                let result = self.sp.wrapping_add_signed(s8 as i16);
                let did_half_carry = (self.sp & 0x0F) + (s8 as u16 & 0x0F) > 0x0F;
                let did_carry = (self.sp & 0xFF) + (s8 as u16 & 0xFF) > 0xFF;

                self.f_reg
                    .flag_reg_set(false, false, did_half_carry, did_carry);

                if opcode == 0xF8 {
                    self.h_reg.combine_and_write(&mut self.l_reg, result);
                } else {
                    self.t_cycle_add(4);
                    self.sp = result;
                }
            }

            // LD SP, HL
            0xF9 => {
                self.t_cycle_add(4);

                self.sp = self.h_reg.combine_and_read(&self.l_reg)
            }

            // DI
            0xF3 => {
                self.ime = 0;
                self.ei_pending = false;
            }

            // EI
            0xFB => self.ei_pending = true,

            _ => unimplemented!("Operation code not recognized. Code: {}", opcode),
        }
    }
}

// PATTERN MATCHING
impl Gameboy {
    fn read_r8_pattern(&mut self, source: u8) -> u8 {
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

    fn read_r16mem_pattern(&mut self, source: u8) -> u8 {
        match source {
            0 => self.read_bus(self.b_reg.combine_and_read(&self.c_reg) as usize),
            1 => self.read_bus(self.d_reg.combine_and_read(&self.e_reg) as usize),
            2 => {
                let val = self.read_bus(self.h_reg.combine_and_read(&self.l_reg) as usize);
                self.h_reg.combine_and_increment(&mut self.l_reg);
                val
            }
            3 => {
                let val = self.read_bus(self.h_reg.combine_and_read(&self.l_reg) as usize);
                self.h_reg.combine_and_decrement(&mut self.l_reg);
                val
            }
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
            }
            3 => {
                self.write_bus(self.h_reg.combine_and_read(&self.l_reg) as usize, value);
                self.h_reg.combine_and_decrement(&mut self.l_reg);
            }
            _ => unreachable!(
                "R16_MEMORY byte matching error. Destination: {}",
                destination
            ),
        }
    }

    fn check_flag_cond(&self, cond: u8) -> bool {
        return match cond {
            0 => self.f_reg.get_zero_flag() == 0,
            1 => self.f_reg.get_zero_flag() == 1,
            2 => self.f_reg.get_carry_flag() == 0,
            3 => self.f_reg.get_carry_flag() == 1,
            _ => unreachable!("Invalid flag condition. Condition: {}", cond),
        };
    }

    fn read_bus(&mut self, address: usize) -> u8 {
        self.t_cycle_add(4);

        match address {
            ROM_BANK_0..ROM_BANK_1 => self.rom.data[address - ROM_BANK_0],
            ROM_BANK_1..VRAM_START => self.rom.data[address - ROM_BANK_0],
            VRAM_START..EXT_RAM_START => {
                if self.ppu.mode == Mode3 {
                    return 0xFF;
                }

                self.vram[address - VRAM_START]
            }
            EXT_RAM_START..WRAM_1_START => self.rom.external_ram[address - EXT_RAM_START],
            WRAM_1_START..WRAM_2_START => self.wram_1[address - WRAM_1_START],
            WRAM_2_START..ECH_RAM_START => self.wram_2[address - WRAM_2_START],
            ECH_RAM_START..OAM_START => {
                let addr = address - 0x2000;

                if addr < WRAM_2_START {
                    self.wram_1[addr - WRAM_1_START]
                } else {
                    self.wram_2[addr - WRAM_2_START]
                }
            }
            OAM_START..UNUSABLE_START => self.oam[address - OAM_START],
            UNUSABLE_START..IO_REGISTERS_START => 0x00,

            // TEMP
            INTERRUPT_FLAG => self.read_io(address),
            INTERRUPT_ENABLE => self.read_io(address),
            0xFF00 => 0xFF, // temp
            IO_REGISTERS_START..H_RAM_START => self.read_io(address),

            H_RAM_START..INTERRUPT_ENABLE => self.hram[address - H_RAM_START],
            _ => unreachable!("Invalid memory address. Address: {}", address),
        }
    }

    fn write_bus(&mut self, address: usize, value: u8) {
        self.t_cycle_add(4);

        match address {
            ROM_BANK_0..VRAM_START => {
                println!("wtf");
            }
            VRAM_START..EXT_RAM_START => {
                if self.ppu.mode != Mode3 {
                    self.vram[address - VRAM_START] = value
                }
            }
            EXT_RAM_START..WRAM_1_START => self.rom.external_ram[address - EXT_RAM_START] = value,
            WRAM_1_START..WRAM_2_START => self.wram_1[address - WRAM_1_START] = value,
            WRAM_2_START..ECH_RAM_START => self.wram_2[address - WRAM_2_START] = value,
            ECH_RAM_START..OAM_START => {
                let addr = address - 0x2000;

                if addr < WRAM_2_START {
                    self.wram_1[addr - WRAM_1_START] = value
                } else {
                    self.wram_2[addr - WRAM_2_START] = value
                }
            }
            OAM_START..UNUSABLE_START => self.oam[address - OAM_START] = value,
            UNUSABLE_START..IO_REGISTERS_START => {}

            // TEMP
            INTERRUPT_FLAG => self.write_io(address, value),
            INTERRUPT_ENABLE => self.write_io(address, value),
            IO_REGISTERS_START..H_RAM_START => {
                if value == 0x81 && address == 0xFF02 {
                    print!("{}", self.read_bus(0xFF01) as char);
                    io::stdout().flush().unwrap();
                    self.write_bus(0xFF02, 0);
                } else {
                    self.write_io(address, value);
                }
            }

            H_RAM_START..INTERRUPT_ENABLE => self.hram[address - H_RAM_START] = value,
            _ => unreachable!("Invalid memory address. Address: {}", address),
        }
    }

    fn read_io(&self, address: usize) -> u8 {
        match address {
            // TIMERS
            DIV => self.get_m_cycles(),
            TIMA => self.tima,
            TMA => self.tma,
            TAC => self.tac,

            // GRAPHICS
            STAT => self.stat,
            LCDC => self.lcdc,
            LY => self.ly,
            LYC => self.lyc,
            SCX => self.scx,
            SCY => self.scy,
            WY => self.wy,
            WX => self.wx,

            // INTERRUPTS
            INTERRUPT_FLAG => self.interrupt_flag,
            INTERRUPT_ENABLE => self.interrupt_enable,
            // _ => unreachable!("Invalid IO memory address. Address: {}", address)
            _ => 0x0,
        }
    }

    fn write_io(&mut self, address: usize, value: u8) {
        match address {
            // TIMERS
            DIV => self.sysclck = 0,
            TIMA => self.tima = value,
            TMA => self.tma = value,
            TAC => self.tac = value,

            // GRAPHICS
            STAT => self.stat = value,
            LCDC => self.lcdc = value,
            LY => {}
            LYC => {
                self.lyc = value;
                if (self.stat & 0b0100_0000) > 0 && self.ly == self.lyc {
                    self.interrupt_flag |= 0b0000_0010;
                }
            }
            SCX => self.scx = value,
            SCY => self.scy = value,
            WY => self.wy = value,
            WX => self.wx = value,

            // INTERRUPTS
            INTERRUPT_FLAG => self.interrupt_flag = value,
            INTERRUPT_ENABLE => self.interrupt_enable = value,
            // _ => unreachable!("Invalid IO memory address. Address: {}", address)
            _ => {}
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

        let did_half_carry = (operand & 0x0F) + carry_flag > a & 0x0F;
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
        let r8_pattern = self.read_r8_pattern(r8);
        let (result, sub_flag, did_half_carry, _) = self.add_8_bit(r8_pattern, 1);
        self.write_r8_pattern(r8, result);
        self.f_reg
            .flag_reg_set(result == 0, sub_flag, did_half_carry, None);
    }

    fn inc_sp(&mut self) {
        self.sp = self.sp.wrapping_add(1);
    }

    fn dec_8_bit(&mut self, r8: u8) {
        let r8_pattern = self.read_r8_pattern(r8);
        let (result, sub_flag, did_half_carry, _) = self.sub_8_bit(r8_pattern, 1);
        self.write_r8_pattern(r8, result);
        self.f_reg
            .flag_reg_set(result == 0, sub_flag, did_half_carry, None);
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

        self.f_reg
            .flag_reg_set(!rlca && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    fn rrc_8_bit(&mut self, r8: u8, rrca: bool) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = val & 0b0000_0001;
        let result = (val >> 1) | (shifted_bit << 7);

        self.f_reg
            .flag_reg_set(!rrca && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    fn rl_8_bit(&mut self, r8: u8, rla: bool) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = (val & 0b1000_0000) >> 7;
        let result = (val << 1) | self.f_reg.get_carry_flag();

        self.f_reg
            .flag_reg_set(!rla && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    fn rr_8_bit(&mut self, r8: u8, rra: bool) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = val & 0b0000_0001;
        let result = (val >> 1) | (self.f_reg.get_carry_flag() << 7);

        self.f_reg
            .flag_reg_set(!rra && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    fn sla_8_bit(&mut self, r8: u8) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = (val & 0b1000_0000) >> 7;
        let result = val << 1;

        self.f_reg
            .flag_reg_set(result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    fn sra_8_bit(&mut self, r8: u8) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = val & 0b0000_0001;
        let result = (val >> 1) | (val & (0b1000_0000));

        self.f_reg
            .flag_reg_set(result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    fn srl_8_bit(&mut self, r8: u8) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = val & 0b0000_0001;
        let result = val >> 1;

        self.f_reg
            .flag_reg_set(result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    fn swap(&mut self, r8: u8) {
        let val = self.read_r8_pattern(r8);
        let result = ((val & 0b0000_1111) << 4) | (val >> 4);

        self.write_r8_pattern(r8, result);
        self.f_reg.flag_reg_set(result == 0, false, false, false);
    }

    fn jr_s8(&mut self, s8: i8) {
        self.pc = self.pc.wrapping_add_signed(s8 as i16)
    }

    fn return_from_subroutine(&mut self) {
        self.t_cycle_add(4);
        let return_addr_low = self.pop_stack();
        let return_addr_high = self.pop_stack();

        let return_addr = combine_bytes(return_addr_high, return_addr_low);
        self.pc = return_addr;
    }

    fn call_a16(&mut self, call_instr: u16) {
        // 12 T - Cycles
        self.push_stack((self.pc >> 8) as u8);
        self.push_stack((self.pc & 0x00FF) as u8);
        self.pc = call_instr;
    }

    fn pop_stack(&mut self) -> u8 {
        let val = self.read_bus(self.sp as usize);
        self.inc_sp();
        val
    }

    fn push_stack(&mut self, value: u8) {
        self.t_cycle_add(2);
        self.dec_sp();
        self.write_bus(self.sp as usize, value);
    }
}

// INTERRUPT HANDLING
impl Gameboy {
    fn call_interrupt_handler(&mut self) {
        let pending = self.interrupt_enable & self.interrupt_flag;
        self.ime = 0;

        for n in 0..=4 {
            if (pending & (0b0000_0001 << n)) > 0 {
                // 0 - vblank, 1 - lcd, 2 - timer, 3 - serial, 4 - joypad
                self.t_cycle_add(8);

                let address = 0x40 + (0x08 * n);
                self.interrupt_flag = self.interrupt_flag & !(0b0000_0001 << n);
                self.call_a16(address);
                break;
            }
        }
    }

    fn has_pending_interrupt(&self) -> bool {
        (self.interrupt_enable & self.interrupt_flag & 0b0001_1111) != 0
    }
}

// TIMERS
impl Gameboy {
    fn get_m_cycles(&self) -> u8 {
        (self.sysclck >> 8) as u8
    }

    fn get_tac_controls(&self) -> (bool, u8) {
        let enabled = (self.tac & 0b0000_0100) > 0;
        let clock_select = self.tac & 0b0000_0011;

        (enabled, clock_select)
    }

    fn t_cycle_add(&mut self, val: u16) {
        let (tac_enabled, clock_select) = self.get_tac_controls();

        let bit_mask = match clock_select {
            0 => 0b00010_0000_0000,
            1 => 0b0000_1000,
            2 => 0b0010_0000,
            3 => 0b1000_0000,
            _ => unreachable!(),
        };

        for _ in 0..val {
            let old_clock = self.sysclck;
            self.sysclck = self.sysclck.wrapping_add(1);
            self.ppu_stuff();

            if tac_enabled {
                let new_clock = self.sysclck;

                if (old_clock & bit_mask) > 0 && (new_clock & bit_mask) == 0 {
                    let (result, did_overflow) = self.tima.overflowing_add(1);

                    if did_overflow {
                        self.tima = self.tma;
                        self.interrupt_flag = self.interrupt_flag | 0b0000_0100;
                    } else {
                        self.tima = result;
                    }
                }
            }
        }
    }
}

// PPU / Graphics
impl Gameboy {
    fn tile_fetch(&self) -> u16 {
        let bg_tile_map_base: u16 = if (self.lcdc & 0b0000_1000) >> 3 == 1 {
            0x9C00
        } else {
            0x9800
        };
        let (tile_data_area, signed_address) = if (self.lcdc & 0b0001_0000) >> 4 == 1 {
            (0x8000 as u16, false)
        } else {
            (0x9000 as u16, true)
        };

        let tilemap_x: u16 = ((self.scx as u16 / 8) + self.ppu.fetcher_x as u16) & 0x1F;
        let tilemap_y: u16 = ((self.ly.wrapping_add(self.scy)) & 0xFF) as u16;

        let tilemap_id = self.vram
            [(bg_tile_map_base + ((tilemap_y / 8) * 32) + tilemap_x) as usize - VRAM_START];

        let tile_data_addr;
        if signed_address {
            tile_data_addr = tile_data_area.wrapping_add_signed(tilemap_id as i8 as i16 * 16)
                as usize
                - VRAM_START;
        } else {
            tile_data_addr =
                tile_data_area.wrapping_add(tilemap_id as u16 * 16) as usize - VRAM_START;
        }

        let tile_offset = (((self.ly.wrapping_add(self.scy)) % 8) * 2) as usize;
        let tile_data_low = self.vram[tile_data_addr + tile_offset];
        let tile_data_high = self.vram[(tile_data_addr) + 1 + tile_offset];

        tile_data_reading(tile_data_low, tile_data_high)
    }

    fn enter_mode(&mut self, mode_entered: PPUMode) {
        self.ppu.mode = mode_entered;

        let will_issue_interrupt = match self.ppu.mode {
            PPUMode::Mode0 => (self.stat & 0b0000_1000) > 0,
            PPUMode::Mode1 => (self.stat & 0b0001_0000) > 0,
            PPUMode::Mode2 => (self.stat & 0b0010_0000) > 0,
            _ => false,
        };

        let mode_bits = match self.ppu.mode {
            PPUMode::Mode0 => 0,
            PPUMode::Mode1 => 1,
            PPUMode::Mode2 => 2,
            PPUMode::Mode3 => 3,
        };

        self.stat = (self.stat & 0b1111_1100) | mode_bits;
        if will_issue_interrupt {
            self.interrupt_flag |= 0b0000_0010;
        }
    }

    fn ppu_stuff(&mut self) {
        if (self.lcdc & 0b1000_0000) == 0 {
            self.ppu.dots = 0;
            self.ly = 0;
            self.ppu.fetcher_x = 0;
            self.enter_mode(PPUMode::Mode0);
            self.ppu.fifo = PixelFifo {
                pixel_fifo: [0; 16],
                fetched_tile_data: 0,
                lx: 0,
                len: 0,
                discarded_pixels: 0,
            };
            return;
        }

        self.ppu.dots += 1;

        if self.ppu.mode == PPUMode::Mode2 {
            if self.ppu.dots == 80 {
                self.enter_mode(PPUMode::Mode3);
                self.ppu.fifo.discarded_pixels = self.scx % 8;
                self.ppu.fetcher_x = 0;
            }
        } else if self.ppu.mode == PPUMode::Mode3 {
            // Mode 3
            if self.ppu.fifo.len <= 8 {
                let fifo_len = self.ppu.fifo.len;
                if fifo_len <= 8 {
                    let tile = self.tile_fetch();
                    self.ppu.fetcher_x += 1;

                    for (i, n) in (fifo_len..fifo_len + 8).enumerate() {
                        self.ppu.fifo.pixel_fifo[n as usize] =
                            ((tile >> (16 - ((i + 1) * 2))) & 0b11) as u8;
                    }

                    self.ppu.fifo.len += 8;
                }
            }

            if self.ppu.fifo.len > 8 {
                if self.ppu.fifo.discarded_pixels > 0 {
                    self.ppu.fifo.discarded_pixels -= 1;
                } else {
                    // push
                    self.screen_buffer
                        [(self.ppu.fifo.lx) as usize + (self.ly as usize * SCREEN_WIDTH)] =
                        pallete_conversion(self.ppu.fifo.pixel_fifo[0]);
                    self.ppu.fifo.lx += 1;
                }
                self.ppu.fifo.pixel_fifo.copy_within(1.., 0);
                self.ppu.fifo.len -= 1;

                if self.ppu.fifo.lx == 160 {
                    self.enter_mode(PPUMode::Mode0);

                    self.ppu.fetcher_x = 0;
                    self.ppu.fifo = PixelFifo {
                        pixel_fifo: [0; 16],
                        fetched_tile_data: 0,
                        lx: 0,
                        len: 0,
                        discarded_pixels: 0,
                    }
                }
            }
        }

        // End of Scanline
        if self.ppu.dots == 456 {
            if self.ppu.mode == PPUMode::Mode1 && self.ly == 0 {
                self.enter_mode(PPUMode::Mode2);
            } else {
                self.ly += 1;
            }

            if self.wy == self.ly {
                self.ppu.y_condition = true;
            }

            if (self.stat & 0b0100_0000) > 0 && self.ly == self.lyc {
                self.interrupt_flag |= 0b0000_0010;
            }

            self.ppu.dots = 0;
            self.ppu.fetcher_x = 0;

            if self.ly == 144 {
                self.interrupt_flag |= 0b0000_0001;
                self.enter_mode(PPUMode::Mode1);
                self.ppu.y_condition = false;
            } else if self.ly == 153 {
                self.ly = 0;
            } else if self.ly < 144 {
                self.enter_mode(PPUMode::Mode2);
            }
        }
    }
}
