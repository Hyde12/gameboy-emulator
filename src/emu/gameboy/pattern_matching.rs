use super::*;

// PATTERN MATCHING
impl Gameboy {
    pub(super) fn read_r8_pattern(&mut self, source: u8) -> u8 {
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

    pub(super) fn write_r8_pattern(&mut self, destination: u8, value: u8) {
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

    pub(super) fn read_r16_pattern(&mut self, source: u8) -> u16 {
        match source {
            0 => self.b_reg.combine_and_read(&self.c_reg),
            1 => self.d_reg.combine_and_read(&self.e_reg),
            2 => self.h_reg.combine_and_read(&self.l_reg),
            3 => self.sp,
            _ => unreachable!("R16 byte matching error. Source: {}", source),
        }
    }

    pub(super) fn write_r16_pattern(&mut self, destination: u8, value: u16) {
        match destination {
            0 => self.b_reg.combine_and_write(&mut self.c_reg, value),
            1 => self.d_reg.combine_and_write(&mut self.e_reg, value),
            2 => self.h_reg.combine_and_write(&mut self.l_reg, value),
            3 => self.sp = value,
            _ => unreachable!("R16 byte matching error. Destination: {}", destination),
        }
    }

    pub(super) fn read_r16mem_pattern(&mut self, source: u8) -> u8 {
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

    pub(super) fn write_r16mem_pattern(&mut self, destination: u8, value: u8) {
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

    pub(super) fn check_flag_cond(&self, cond: u8) -> bool {
        return match cond {
            0 => self.f_reg.get_zero_flag() == 0,
            1 => self.f_reg.get_zero_flag() == 1,
            2 => self.f_reg.get_carry_flag() == 0,
            3 => self.f_reg.get_carry_flag() == 1,
            _ => unreachable!("Invalid flag condition. Condition: {}", cond),
        };
    }

    pub(super) fn read_bus(&mut self, address: usize) -> u8 {
        self.t_cycle_add(4);

        match address {
            ROM_BANK_0..ROM_BANK_1 => self.rom.data[address - ROM_BANK_0],
            ROM_BANK_1..VRAM_START => self.rom.data[address - ROM_BANK_0],
            VRAM_START..EXT_RAM_START => {
                if self.ppu.mode == PPUMode::Mode3 {
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

    pub(super) fn write_bus(&mut self, address: usize, value: u8) {
        self.t_cycle_add(4);

        match address {
            ROM_BANK_0..VRAM_START => {
                println!("wtf");
            }
            VRAM_START..EXT_RAM_START => {
                if self.ppu.mode != PPUMode::Mode3 {
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
