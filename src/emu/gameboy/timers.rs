use super::*;

// INTERRUPT HANDLING
impl Gameboy {
    pub(super) fn call_interrupt_handler(&mut self) {
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

    pub(super) fn has_pending_interrupt(&self) -> bool {
        (self.interrupt_enable & self.interrupt_flag & 0b0001_1111) != 0
    }
}

// TIMERS
impl Gameboy {
    pub(super) fn get_m_cycles(&self) -> u8 {
        (self.sysclck >> 8) as u8
    }

    fn get_tac_controls(&self) -> (bool, u8) {
        let enabled = (self.tac & 0b0000_0100) > 0;
        let clock_select = self.tac & 0b0000_0011;

        (enabled, clock_select)
    }

    pub(super) fn t_cycle_add(&mut self, val: u16) {
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
