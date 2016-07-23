use RegT;
use bus::Bus;

pub const A : usize = 0;    // PIO channel A
pub const B : usize = 1;    // PIO channel B
pub const NUM_CHANNELS : usize = 2;

#[derive(Clone, Copy, PartialEq)]
enum Expect {
    Any,
    IOSelect,
    IntMask,
}

pub const MODE_OUTPUT          : u8 = 0;
pub const MODE_INPUT           : u8 = 1;
pub const MODE_BIDIRECTIONAL   : u8 = 2;
pub const MODE_BITCONTROL      : u8 = 3;

pub const INTCTRL_ENABLE_INT   : u8 = (1<<7);
pub const INTCTRL_AND_OR       : u8 = (1<<6);
pub const INTCTRL_HIGH_LOW     : u8 = (1<<5);
pub const INTCTRL_MASK_FOLLOWS : u8 = (1<<4);

#[derive(Clone, Copy)]
struct Channel {
    pub expect : Expect,        // next expected control byte type
    pub mode : u8,              // current operation mode
    pub output: u8,             // output register value
    pub input: u8,              // input register value
    pub io_select: u8,          // IO select bits for bit-control mode
    pub int_mask : u8,
    pub int_vector : u8,
    pub int_control : u8,
    pub bctrl_match : bool,
    pub rdy : bool,
    pub stb : bool,
}

/// the Z80 PIO state
pub struct PIO {
    id : usize,     // id of PIO (needed for systems with multiple ids)
    chn : [Channel; NUM_CHANNELS]
}

impl PIO {
    /// initialize new PIO object
    pub fn new(id: usize) -> PIO {
        PIO {
            id: id,
            chn: [
                Channel {
                    expect:         Expect::Any,
                    mode:           MODE_OUTPUT,
                    output:         0,
                    input:          0,
                    io_select:      0,
                    int_mask:       0xFF,
                    int_vector:     0,
                    int_control:    0,
                    bctrl_match:    false,
                    rdy:            false,
                    stb:            false,
                }; NUM_CHANNELS
            ]
        }
    }

    /// reset the PIO
    pub fn reset(&mut self) {
        for chn in self.chn.iter_mut() {
            chn.mode   = MODE_INPUT;
            chn.expect = Expect::Any;
            chn.output = 0;
            chn.io_select = 0;
            chn.int_mask  = 0xFF;
            chn.int_control &= !INTCTRL_ENABLE_INT;
            chn.bctrl_match = false;
            chn.rdy = false;
            chn.stb = false;
        }
    }

    /// write to control register
    pub fn write_control(&mut self, chn: usize, val: RegT) {
        let c = &mut self.chn[chn];
        match c.expect {
            Expect::IOSelect => {
                c.io_select = val as u8;
                c.expect = Expect::Any;
            },
            Expect::IntMask => {
                c.int_mask = val as u8;
                c.expect = Expect::Any;
            },
            Expect::Any => {
                match val & 0xF {
                    // set channel mode
                    0xF => {
                        let m = ((val>>6) & 3) as u8;
                        if (chn == B) && m == MODE_BIDIRECTIONAL {
                            panic!("MODE_BIDIRECTIONAL on PIO channel B not allowed!");
                        }
                        else {
                            c.mode = m;
                            if m == MODE_BITCONTROL {
                                c.expect = Expect::IOSelect;
                                c.bctrl_match = false;
                            }
                        }
                    },
                    // set interrupt control word
                    0x7 => {
                        c.int_control = (val & 0xF0) as u8;
                        if (val as u8 & INTCTRL_MASK_FOLLOWS) != 0 {
                            c.expect = Expect::IntMask;
                            c.bctrl_match = false;
                        }
                    },
                    // set/clear interrupt enable bit
                    0x3 => {
                        c.int_control = ((val as u8) & INTCTRL_ENABLE_INT) |
                                        (c.int_control & !INTCTRL_ENABLE_INT);
                    },
                    // set interrupt vector
                    _ if (val & 1) == 0 => {
                        c.int_vector = val as u8;
                    },
                    _ => panic!("Invalid PIO control word!")
                }
            }
        }
    }

    /// read control register
    pub fn read_control(&self) -> RegT {
        ((self.chn[A].int_control & 0xC0) | (self.chn[B].int_control >> 4)) as RegT
    }

    /// set rdy flag on channel, and call pio_rdy callback on bus if changed
    fn set_rdy(&mut self, bus: &Bus, chn: usize, rdy: bool) {
        let c = &mut self.chn[chn];
        if c.rdy != rdy {
            c.rdy = rdy;
            bus.pio_rdy(self.id, chn, rdy);
        }
    }

    /// write data to PIO channel
    pub fn write_data(&mut self, bus: &Bus, chn: usize, data: RegT) {
        match self.chn[chn].mode {
            MODE_OUTPUT => {
                self.set_rdy(bus, chn, false);
                self.chn[chn].output = data as u8;
                bus.pio_outp(self.id, chn, data);
                self.set_rdy(bus, chn, true);
            },
            MODE_INPUT => { 
                self.chn[chn].output = data as u8;  // not a bug
            },
            MODE_BIDIRECTIONAL => {
                self.set_rdy(bus, chn, false);
                self.chn[chn].output = data as u8;
                if !self.chn[chn].stb {
                    bus.pio_outp(self.id, chn, data);
                }
                self.set_rdy(bus, chn, true);
            },
            MODE_BITCONTROL => {
                self.chn[chn].output = data as u8;
                bus.pio_outp(self.id, chn, data);
            },
            _ => panic!("Invalid PIO mode!")
        }
    }

    /// read data from PIO channel
    pub fn read_data(&mut self, bus: &Bus, chn: usize) -> RegT {
        match self.chn[chn].mode {
            MODE_OUTPUT => {
                self.chn[chn].output as RegT
            },
            MODE_INPUT => {
                if !self.chn[chn].stb {
                    self.chn[chn].input = bus.pio_inp(self.id, chn) as u8;
                }
                self.set_rdy(bus, chn, false);
                self.set_rdy(bus, chn, true);
                self.chn[chn].input as RegT
            },
            MODE_BIDIRECTIONAL => {
                self.set_rdy(bus, chn, false);
                self.set_rdy(bus, chn, true);
                self.chn[chn].input as RegT
            },
            MODE_BITCONTROL => {
                self.chn[chn].input = bus.pio_inp(self.id, chn) as u8;
                let c = self.chn[chn];
                ((c.input & c.io_select) | (c.output & !c.io_select)) as RegT
            }
            _ => panic!("Invalid PIO mode!")
        }
    }

    /// write data from peripheral device into PIO
    pub fn write(&mut self, bus: &Bus, chn: usize, data: RegT) {
        let mut c = self.chn[chn];
        if c.mode == MODE_BITCONTROL {
            c.input = data as u8;
            let mask = !c.int_mask;
            let val = mask & ((c.input & c.io_select) | (c.output & !c.io_select));
            let ictrl = c.int_control & 0x60;

            let bmatch = 
                ((ictrl == 0x00) && (val != mask)) ||
                ((ictrl == 0x20) && (val != 0)) ||
                ((ictrl == 0x40) && (val == 0)) ||
                ((ictrl == 0x60) && (val == mask));
            
            if !c.bctrl_match && bmatch && (0 != (c.int_control & INTCTRL_ENABLE_INT)) {
                bus.pio_int(self.id, chn, c.int_vector as RegT);
            }
            c.bctrl_match = bmatch;
        }
    }
}

//------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use pio::Expect;

    #[test]
    fn reset() {
        let mut pio = PIO::new(0);
        for chn in pio.chn.iter() {
            assert!(Expect::Any == chn.expect);
            assert!(MODE_OUTPUT == chn.mode);
            assert!(0 == chn.output);
            assert!(0 == chn.input);
            assert!(0 == chn.io_select);
            assert!(0xFF == chn.int_mask);
            assert!(0 == chn.int_vector);
            assert!(0 == chn.int_control); 
            assert!(!chn.bctrl_match);
        }
        pio.chn[A].mode         = MODE_BIDIRECTIONAL;
        pio.chn[A].expect       = Expect::IntMask;
        pio.chn[A].output       = 0x12;
        pio.chn[A].input        = 0x34;
        pio.chn[A].io_select    = 0xAA;
        pio.chn[A].int_mask     = 0xEE;
        pio.chn[A].int_vector   = 0x10;
        pio.chn[A].int_control  = INTCTRL_ENABLE_INT|INTCTRL_MASK_FOLLOWS;
        pio.chn[A].bctrl_match  = true;
        pio.chn[B].mode         = MODE_BITCONTROL;
        pio.chn[B].expect       = Expect::IOSelect;
        pio.chn[B].output       = 0x56;
        pio.chn[B].input        = 0x78;
        pio.chn[B].io_select    = 0xBB;
        pio.chn[B].int_mask     = 0x55;
        pio.chn[B].int_vector   = 0x20;
        pio.chn[B].int_control  = INTCTRL_ENABLE_INT|INTCTRL_HIGH_LOW;
        pio.chn[B].bctrl_match  = true;
        
        pio.reset();
        for chn in pio.chn.iter() {
            assert!(MODE_INPUT == chn.mode);
            assert!(Expect::Any == chn.expect);
            assert!(0 == chn.output);
            assert!(0 == chn.io_select);
            assert!(0xFF == chn.int_mask);            
            assert!(0 == (chn.int_control & INTCTRL_ENABLE_INT));
            assert!(!chn.bctrl_match);
        }
        assert!(0x34 == pio.chn[A].input);
        assert!(0x78 == pio.chn[B].input);
        assert!(0x10 == pio.chn[A].int_vector);
        assert!(0x20 == pio.chn[B].int_vector);
        assert!(INTCTRL_MASK_FOLLOWS == pio.chn[A].int_control);
        assert!(INTCTRL_HIGH_LOW == pio.chn[B].int_control);
    }   

    #[test]
    fn write_control() {
        let mut pio = PIO::new(0);

        // load interrupt vector (bit 0 == 0)
        pio.write_control(A, 0xE0);
        pio.write_control(B, 0xE2);
        assert!(0xE0 == pio.chn[A].int_vector);
        assert!(0xE2 == pio.chn[B].int_vector);

        // set operating modes (0bmmxx1111), where mm
        // is the mode (00:output, 01:input, 10:bidirectional, 11:bitcontrol)
        // xx is ignored
        // bidirectional requires the bit control word to be written next
        pio.write_control(A, 0b00101111);   // output
        assert!(MODE_OUTPUT == pio.chn[A].mode);
        pio.write_control(A, 0b01011111);   // input
        assert!(MODE_INPUT == pio.chn[A].mode);
        pio.write_control(A, 0b10111111);   // bidirectional
        assert!(MODE_BIDIRECTIONAL == pio.chn[A].mode);
        pio.write_control(A, 0b11001111);   // bitcontrol
        assert!(MODE_BITCONTROL == pio.chn[A].mode);
        assert!(Expect::IOSelect == pio.chn[A].expect);
        pio.write_control(A, 0b10101010);   // write bitcontrol IO mask
        assert!(0b10101010 == pio.chn[A].io_select);
        assert!(Expect::Any == pio.chn[A].expect);

        // set interrupt control word
        // bit 7: interrupt enable/disable
        // bit 6: logic and/or (bitcontrol mode)
        // bit 5: high/low (bitcontrol mode)
        // bit 4: mask follows (bitcontrol mode)
        // bit 3..0: 0111
        pio.write_control(A, 0b10100111);
        assert!(0b10100000 == pio.chn[A].int_control);
        assert!(Expect::Any == pio.chn[A].expect);
        assert!(INTCTRL_ENABLE_INT|INTCTRL_HIGH_LOW == 
                INTCTRL_ENABLE_INT|INTCTRL_HIGH_LOW & pio.chn[A].int_control);
        pio.write_control(A, 0b00010111);
        assert!(0b00010000 == pio.chn[A].int_control);
        assert!(INTCTRL_MASK_FOLLOWS == pio.chn[A].int_control & INTCTRL_MASK_FOLLOWS);
        assert!(Expect::IntMask == pio.chn[A].expect);
        pio.write_control(A, 0b01010101);
        assert!(0b01010101 == pio.chn[A].int_mask);
        assert!(Expect::Any == pio.chn[A].expect);

        // set interrupt enable bit individually
        pio.write_control(A, 0b11100111);
        assert!(0b11100000 == pio.chn[A].int_control);
        pio.write_control(A, 0b00000011);
        assert!(0b01100000 == pio.chn[A].int_control);
        pio.write_control(A, 0b10110011);
        assert!(0b11100000 == pio.chn[A].int_control);
        assert!(Expect::Any == pio.chn[A].expect);
    }
}

