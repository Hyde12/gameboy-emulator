pub fn combine_bytes(left_byte: u8, right_byte: u8) -> u16 {
    ((left_byte as u16) << 8) | (right_byte as u16)
}
