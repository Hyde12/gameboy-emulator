pub trait Register8 {
    fn new(value: u8) -> Self;
    fn set(&mut self, value: u8);
    fn read(&self) -> u8;

    fn combine_and_write(&mut self, other_reg: &mut Register, new_val: u16) {
        self.set(((new_val & 0xFF00) >> 8) as u8);
        other_reg.set((new_val & 0x00FF) as u8);
    }

    fn combine_and_read() -> u16 {
        todo!()
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
    pub fn zero_flag(&self) -> bool {
        (self.f_reg & 0b0100_0000) != 1
    }

    pub fn subtraction_flag(&self) -> bool {
        (self.f_reg & 0b1000_0000) != 1
    }

    pub fn half_carry_flag(&self) -> bool {
        (self.f_reg & 0b0001_0000) != 1
    }

    pub fn carry_flag(&self) -> bool {
        (self.f_reg & 0b0000_1000) != 1
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
