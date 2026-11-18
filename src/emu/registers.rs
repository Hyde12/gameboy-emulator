pub trait Register8 {
    fn new(value: u8) -> Self;
    fn set(&mut self, value: u8);
    fn read(&self) -> u8;

    fn combine_and_write(&mut self, other_reg: &mut Register, new_val: u16) {
        self.set(((new_val & 0xFF00) >> 8) as u8);
        other_reg.set((new_val & 0x00FF) as u8);
    }

    fn combine_and_read(&self, other_reg: &Register) -> u16 {
        (self.read() as u16) << 8 + other_reg.read()
    }

    fn combine_and_increment(&mut self, other_reg: &mut Register) {
        self.combine_and_write(other_reg, self.combine_and_read(other_reg).wrapping_add(1));
    }

    fn combine_and_decrement(&mut self, other_reg: &mut Register) {
        self.combine_and_write(other_reg, self.combine_and_read(other_reg).wrapping_sub(1));
    }
}

pub struct FlagRegister {
    f_reg: u8,
}

impl Register8 for FlagRegister {
    fn new(value: u8) -> Self {
        FlagRegister { f_reg: value }
    }

    fn set(&mut self, value: u8) {
        self.f_reg = value;
    }

    fn read(&self) -> u8 {
        self.f_reg
    }
}

impl FlagRegister {
    pub fn flag_reg_set(
        &mut self,
        z: impl Into<Option<bool>>,
        n: impl Into<Option<bool>>,
        h: impl Into<Option<bool>>,
        c: impl Into<Option<bool>>,
    ) {
        let z = z.into();
        let n = n.into();
        let h = h.into();
        let c = c.into();

        match z {
            None => {}
            Some(set) => {
                self.set_zero_flag(set);
            }
        }

        match n {
            None => {}
            Some(set) => {
                self.set_subtraction_flag(set);
            }
        }

        match h {
            None => {}
            Some(set) => {
                self.set_half_carry_flag(set);
            }
        }

        match c {
            None => {}
            Some(set) => {
                self.set_carry_flag(set);
            }
        }
    }

    pub fn get_zero_flag(&self) -> u8 {
        (self.f_reg & 0b1000) >> 3
    }

    pub fn set_zero_flag(&mut self, set: bool) {
        if set {
            self.f_reg |= 0b1000
        } else {
            self.f_reg &= 0b0111
        }
    }

    pub fn get_subtraction_flag(&self) -> u8 {
        (self.f_reg & 0b0100) >> 2
    }

    pub fn set_subtraction_flag(&mut self, set: bool) {
        if set {
            self.f_reg |= 0b0100
        } else {
            self.f_reg &= 0b1011
        }
    }

    pub fn get_half_carry_flag(&self) -> u8 {
        (self.f_reg & 0b0010) >> 1
    }

    pub fn set_half_carry_flag(&mut self, set: bool) {
        if set {
            self.f_reg |= 0b0010
        } else {
            self.f_reg &= 0b1101
        }
    }

    pub fn get_carry_flag(&self) -> u8 {
        self.f_reg & 0b0001
    }

    pub fn set_carry_flag(&mut self, set: bool) {
        if set {
            self.f_reg |= 0b0001
        } else {
            self.f_reg &= 0b1110
        }
    }
}

pub struct Register {
    reg: u8,
}

impl Register8 for Register {
    fn new(value: u8) -> Self {
        Register { reg: value }
    }

    fn set(&mut self, value: u8) {
        self.reg = value;
    }

    fn read(&self) -> u8 {
        self.reg
    }
}
