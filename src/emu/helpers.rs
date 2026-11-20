use crate::emu::registers::FlagRegister;

pub fn combine_bytes(left_byte: u8, right_byte: u8) -> u16 {
    ((left_byte as u16) << 8) | (right_byte as u16)
}

pub fn flag_conditions(cond: u8, f_reg: FlagRegister) -> bool {
    match cond {
        0 => { return f_reg.get_zero_flag() == 0 }
        1 => { return f_reg.get_zero_flag() == 1 }
        2 => { return f_reg.get_carry_flag() == 0 }
        3 => { return f_reg.get_carry_flag() == 1 }
        _ => unreachable!(),
    };
}
