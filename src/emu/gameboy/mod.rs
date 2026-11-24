mod cpu_alu;
mod pattern_matching;
mod ppu;
mod timers;

use std::io::{self, Write};

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
    object_pixel_fifo: [u8; 16],
    window_triggered: bool,

    lx: u8,
    len: u8,
    obj_len: u8,
    discarded_pixels: u8,
}

struct PPU {
    dots: u16,
    mode: PPUMode,
    penalty: u8,

    // mode 2
    oam_buffer: [u8; 40],
    object_count: u8,

    // mode 3
    window_y_condition: bool,
    window_y: u8,
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
                window_y_condition: false,
                window_y: 0,
                penalty: 0,
                fifo: PixelFifo {
                    object_pixel_fifo: [0; 16],
                    pixel_fifo: [0; 16],
                    window_triggered: false,
                    lx: 0,
                    obj_len: 0,
                    len: 0,
                    discarded_pixels: 0,
                },
                oam_buffer: [0; 40],
                object_count: 0,
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
