pub fn pallete_conversion(num: u8) -> u32 {
    match num {
        0 => 0x9BBC0F,
        1 => 0x8BAC0F,
        2 => 0x306230,
        3 => 0x0F380F,
        _ => unreachable!()
    }
}

pub fn tile_data_reading(tile_data_low: u8, tile_data_high: u8) -> u16 {
    let mut pixels: u16 = 0;

    for n in 0..8 {
        let shift = 7 - n;
        let tile_bit_high = ((tile_data_high >> shift) & 1) as u16;
        let tile_bit_low = ((tile_data_low >> shift) & 1) as u16;

        pixels = (pixels << 2) | (tile_bit_high << 1) | tile_bit_low
    }

    pixels
}

pub fn combine_bytes(left_byte: u8, right_byte: u8) -> u16 {
    ((left_byte as u16) << 8) | (right_byte as u16)
}
