use super::*;

// ARITHMETIC
impl Gameboy {
    pub(super) fn add_8_bit(&self, r8: u8, operand: u8) -> (u8, bool, bool, bool) {
        let (result, did_carry) = r8.overflowing_add(operand);
        let did_half_carry = ((r8 & 0x0F) + (operand & 0x0F)) > 0x0F;

        (result, false, did_half_carry, did_carry)
    }

    pub(super) fn adc_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        let carry_flag = self.f_reg.get_carry_flag();

        let (a_r8, did_overflow_1) = a.overflowing_add(operand);
        let (a_r8_c, did_overflow_2) = a_r8.overflowing_add(carry_flag);

        let did_half_carry = ((a & 0x0F) + (operand & 0x0F) + carry_flag) > 0x0F;
        let did_carry = did_overflow_1 || did_overflow_2;

        (a_r8_c, false, did_half_carry, did_carry)
    }

    pub(super) fn sub_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        let (result, did_carry) = a.overflowing_sub(operand);
        let did_half_carry = (operand & 0x0F) > (a & 0x0F);

        (result, true, did_half_carry, did_carry)
    }

    pub(super) fn sbc_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        let carry_flag = self.f_reg.get_carry_flag();

        let (a_r8, did_underflow_1) = a.overflowing_sub(operand);
        let (a_r8_c, did_underflow_2) = a_r8.overflowing_sub(carry_flag);

        let did_half_carry = (operand & 0x0F) + carry_flag > a & 0x0F;
        let did_carry = did_underflow_1 || did_underflow_2;

        (a_r8_c, true, did_half_carry, did_carry)
    }

    pub(super) fn and_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        (a & operand, false, true, false)
    }

    pub(super) fn xor_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        (a ^ operand, false, false, false)
    }

    pub(super) fn or_8_bit(&self, a: u8, operand: u8) -> (u8, bool, bool, bool) {
        (a | operand, false, false, false)
    }

    pub(super) fn inc_8_bit(&mut self, r8: u8) {
        let r8_pattern = self.read_r8_pattern(r8);
        let (result, sub_flag, did_half_carry, _) = self.add_8_bit(r8_pattern, 1);
        self.write_r8_pattern(r8, result);
        self.f_reg
            .flag_reg_set(result == 0, sub_flag, did_half_carry, None);
    }

    pub(super) fn inc_sp(&mut self) {
        self.sp = self.sp.wrapping_add(1);
    }

    pub(super) fn dec_8_bit(&mut self, r8: u8) {
        let r8_pattern = self.read_r8_pattern(r8);
        let (result, sub_flag, did_half_carry, _) = self.sub_8_bit(r8_pattern, 1);
        self.write_r8_pattern(r8, result);
        self.f_reg
            .flag_reg_set(result == 0, sub_flag, did_half_carry, None);
    }

    pub(super) fn dec_sp(&mut self) {
        self.sp = self.sp.wrapping_sub(1)
    }
}

// MEMORY MAGIC
impl Gameboy {
    pub(super) fn rlc_8_bit(&mut self, r8: u8, rlca: bool) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = (val & 0b1000_0000) >> 7;
        let result = (val << 1) | shifted_bit;

        self.f_reg
            .flag_reg_set(!rlca && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    pub(super) fn rrc_8_bit(&mut self, r8: u8, rrca: bool) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = val & 0b0000_0001;
        let result = (val >> 1) | (shifted_bit << 7);

        self.f_reg
            .flag_reg_set(!rrca && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    pub(super) fn rl_8_bit(&mut self, r8: u8, rla: bool) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = (val & 0b1000_0000) >> 7;
        let result = (val << 1) | self.f_reg.get_carry_flag();

        self.f_reg
            .flag_reg_set(!rla && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    pub(super) fn rr_8_bit(&mut self, r8: u8, rra: bool) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = val & 0b0000_0001;
        let result = (val >> 1) | (self.f_reg.get_carry_flag() << 7);

        self.f_reg
            .flag_reg_set(!rra && result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    pub(super) fn sla_8_bit(&mut self, r8: u8) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = (val & 0b1000_0000) >> 7;
        let result = val << 1;

        self.f_reg
            .flag_reg_set(result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    pub(super) fn sra_8_bit(&mut self, r8: u8) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = val & 0b0000_0001;
        let result = (val >> 1) | (val & (0b1000_0000));

        self.f_reg
            .flag_reg_set(result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    pub(super) fn srl_8_bit(&mut self, r8: u8) {
        let val = self.read_r8_pattern(r8);
        let shifted_bit = val & 0b0000_0001;
        let result = val >> 1;

        self.f_reg
            .flag_reg_set(result == 0, false, false, shifted_bit == 1);
        self.write_r8_pattern(r8, result);
    }

    pub(super) fn swap(&mut self, r8: u8) {
        let val = self.read_r8_pattern(r8);
        let result = ((val & 0b0000_1111) << 4) | (val >> 4);

        self.write_r8_pattern(r8, result);
        self.f_reg.flag_reg_set(result == 0, false, false, false);
    }

    pub(super) fn jr_s8(&mut self, s8: i8) {
        self.pc = self.pc.wrapping_add_signed(s8 as i16)
    }

    pub(super) fn return_from_subroutine(&mut self) {
        self.t_cycle_add(4);
        let return_addr_low = self.pop_stack();
        let return_addr_high = self.pop_stack();

        let return_addr = combine_bytes(return_addr_high, return_addr_low);
        self.pc = return_addr;
    }

    pub(super) fn call_a16(&mut self, call_instr: u16) {
        // 12 T - Cycles
        self.push_stack((self.pc >> 8) as u8);
        self.push_stack((self.pc & 0x00FF) as u8);
        self.pc = call_instr;
    }

    pub(super) fn pop_stack(&mut self) -> u8 {
        let val = self.read_bus(self.sp as usize);
        self.inc_sp();
        val
    }

    pub(super) fn push_stack(&mut self, value: u8) {
        self.t_cycle_add(2);
        self.dec_sp();
        self.write_bus(self.sp as usize, value);
    }
}
