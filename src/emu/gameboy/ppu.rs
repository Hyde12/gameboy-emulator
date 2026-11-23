use super::*;

// PPU / Graphics
impl Gameboy {
    fn tile_fetch(&self) -> u16 {
        // Check if the current pixel is a WINDOW
        let is_window = self.ppu.fifo.window_triggered && self.lcdc & 0b0010_0000 > 0;

        // set accordinglyl, calculation differs for the tile_data_area depending on which option was picked
        let tile_map_bit = if is_window { 0b0100_0000 } else { 0b0000_1000 };
        let bg_tile_map_base: u16 = if (self.lcdc & tile_map_bit) > 0 {
            0x9C00
        } else {
            0x9800
        };

        let (tile_data_area, signed_address) = if (self.lcdc & 0b0001_0000) >> 4 == 1 {
            (0x8000 as u16, false)
        } else {
            (0x9000 as u16, true)
        };

        // if the current pixel is a window use the current x coord (since the window y condition was activated) otherwise, use the formula
        let tilemap_x: u16 = if is_window {
            self.ppu.fetcher_x as u16
        } else {
            ((self.scx as u16 / 8) + self.ppu.fetcher_x as u16) & 0x1F
        };

        let tilemap_y: u16 = if is_window {
            self.ppu.window_y as u16
        } else {
            ((self.ly.wrapping_add(self.scy)) & 0xFF) as u16
        };

        // get the tilemap id used. found stored in the vram addr of the current tilemap.
        let tilemap_id = self.vram
            [(bg_tile_map_base + ((tilemap_y / 8) * 32) + tilemap_x) as usize - VRAM_START];

        // tile data address calculation depends on the earlier option
        let tile_data_addr;
        if signed_address {
            tile_data_addr = tile_data_area.wrapping_add_signed(tilemap_id as i8 as i16 * 16)
                as usize
                - VRAM_START;
        } else {
            tile_data_addr =
                tile_data_area.wrapping_add(tilemap_id as u16 * 16) as usize - VRAM_START;
        }

        // set an offset to get the row of the tile id we're in
        let tile_offset = ((tilemap_y % 8) * 2) as usize;
        let tile_data_low = self.vram[tile_data_addr + tile_offset];
        let tile_data_high = self.vram[(tile_data_addr) + 1 + tile_offset];

        // read it as a u16
        tile_data_reading(tile_data_low, tile_data_high)
    }

    fn enter_mode(&mut self, mode_entered: PPUMode) {
        // set the ppu to the mode that was entered
        self.ppu.mode = mode_entered;

        // check stat interrupt activation
        let will_issue_interrupt = match self.ppu.mode {
            PPUMode::Mode0 => (self.stat & 0b0000_1000) > 0,
            PPUMode::Mode1 => (self.stat & 0b0001_0000) > 0,
            PPUMode::Mode2 => (self.stat & 0b0010_0000) > 0,
            _ => false,
        };

        // set first two bits of stat depending on yis
        let mode_bits = match self.ppu.mode {
            PPUMode::Mode0 => 0,
            PPUMode::Mode1 => 1,
            PPUMode::Mode2 => 2,
            PPUMode::Mode3 => 3,
        };
        self.stat = (self.stat & 0b1111_1100) | mode_bits;

        // if interrupt matched, set the interrupt flag
        if will_issue_interrupt {
            self.interrupt_flag |= 0b0000_0010;
        }
    }

    pub(super) fn ppu_stuff(&mut self) {
        // check if lcd is off, reset everything if yes
        if (self.lcdc & 0b1000_0000) == 0 {
            self.ppu.dots = 0;
            self.ly = 0;
            self.ppu.fetcher_x = 0;
            self.enter_mode(PPUMode::Mode0);
            self.ppu.window_y = 0;
            self.ppu.fifo = PixelFifo {
                pixel_fifo: [0; 16],
                window_triggered: false,
                lx: 0,
                len: 0,
                discarded_pixels: 0,
            };
            return;
        }

        // increment dot (ppu count, same as a t cycle)
        self.ppu.dots += 1;

        if self.ppu.mode == PPUMode::Mode2 {
            // go to mode 3 after oam stuff
            if self.ppu.dots == 80 {
                // check if it's a window line and activate if true
                if self.wy == self.ly {
                    self.ppu.window_y_condition = true;
                }

                self.enter_mode(PPUMode::Mode3);
                self.ppu.fifo.discarded_pixels = self.scx % 8;
                self.ppu.fetcher_x = 0;
            }
        } else if self.ppu.mode == PPUMode::Mode3 {
            // check if pixel is a window, if the lcdc wants windows activated. saturated sub as wx = x - 7
            if (self.ppu.fifo.lx >= self.wx.saturating_sub(7))
                && self.ppu.window_y_condition
                && !self.ppu.fifo.window_triggered
                && self.lcdc & 0b0010_0000 > 0
            {
                self.ppu.fetcher_x = 0;
                self.ppu.fifo = PixelFifo {
                    pixel_fifo: [0; 16],
                    window_triggered: true,
                    lx: self.ppu.fifo.lx,
                    len: 0,
                    discarded_pixels: 0,
                };

                // in the case that wx is less than 7, discard the pixels equal to the difference of that since technically wx still isnt visible/negative
                if self.wx < 7 {
                    self.ppu.fifo.discarded_pixels = 7 - self.wx;
                }
            }

            // if the fifo doesnt have enough pixels to push get more
            if self.ppu.fifo.len <= 8 {
                let fifo_len = self.ppu.fifo.len;
                let tile = self.tile_fetch();

                // for each tile fetched by our fetcher
                self.ppu.fetcher_x += 1;

                // put the pixels in the fifo
                for (i, n) in (fifo_len..fifo_len + 8).enumerate() {
                    self.ppu.fifo.pixel_fifo[n as usize] =
                        ((tile >> (16 - ((i + 1) * 2))) & 0b11) as u8;
                }

                // as we got 8 more pixels from this fetching operation
                self.ppu.fifo.len += 8;
            }

            // if fifo has enough pixels
            if self.ppu.fifo.len > 8 {
                // dont push the pixels if they are to be discarded
                if self.ppu.fifo.discarded_pixels > 0 {
                    self.ppu.fifo.discarded_pixels -= 1;
                } else {
                    // push to the screen, convert them to colors first. if the bg/window prio is off then just return white/lightest color.
                    self.screen_buffer
                        [(self.ppu.fifo.lx) as usize + (self.ly as usize * SCREEN_WIDTH)] =
                        pallete_conversion(if self.lcdc & 0b0000_0001 == 1 {
                            self.ppu.fifo.pixel_fifo[0]
                        } else {
                            0
                        });
                    self.ppu.fifo.lx += 1;
                }

                // "shifts" the pixels so the one pushed to the screen is removed
                self.ppu.fifo.pixel_fifo.copy_within(1.., 0);
                self.ppu.fifo.len -= 1;

                // when we finish displaying all the pixels for the row
                if self.ppu.fifo.lx == 160 {
                    // go to the next mode, reset everything
                    self.enter_mode(PPUMode::Mode0);
                    self.ppu.fetcher_x = 0;

                    // if the window y was triggered increment it
                    if self.ppu.fifo.window_triggered {
                        self.ppu.window_y += 1;
                    }

                    // reset fifo
                    self.ppu.fifo = PixelFifo {
                        pixel_fifo: [0; 16],
                        window_triggered: false,
                        lx: 0,
                        len: 0,
                        discarded_pixels: 0,
                    }
                }
            }
        }

        // End of Scanline
        if self.ppu.dots == 456 {
            // if wy and ly are the same, window y condition is activated
            if self.wy == self.ly {
                self.ppu.window_y_condition = true;
            }

            if self.ppu.mode == PPUMode::Mode1 && self.ly == 0 {
                // edge case because vblank sets ly to 0 a bit early
                self.enter_mode(PPUMode::Mode2);
            } else {
                // increment ly
                self.ly += 1;
            }

            // ly == lyc stat interrupt
            if (self.stat & 0b0100_0000) > 0 && self.ly == self.lyc {
                self.interrupt_flag |= 0b0000_0010;
            }

            // reset dots and fetched tile count
            self.ppu.dots = 0;
            self.ppu.fetcher_x = 0;

            if self.ly == 144 {
                // vblank interrupt and enter vblank
                self.interrupt_flag |= 0b0000_0001;
                self.enter_mode(PPUMode::Mode1);

                // reset window conditions and stuff
                self.ppu.window_y = 0;
                self.ppu.window_y_condition = false;
            } else if self.ly == 153 {
                // go to ly a few steps before end of vblank
                self.ly = 0;
            } else if self.ly < 144 {
                // after every scanline default to mode 2
                self.enter_mode(PPUMode::Mode2);
            }
        }
    }
}
