#![allow(unused)]
use RegT;
use bus::Bus;

pub const CTC_0: usize = 0;
pub const CTC_1: usize = 1;
pub const CTC_2: usize = 2;
pub const CTC_3: usize = 3;
const NUM_CHANNELS: usize = 4;

pub const CTC_INTERRUPT_BIT: u8 = 1 << 7;
pub const CTC_INTERRUPT_ENABLED: u8 = CTC_INTERRUPT_BIT;
pub const CTC_INTERRUPT_DISABLED: u8 = 0;

pub const CTC_MODE_BIT: u8 = 1 << 6;
pub const CTC_MODE_COUNTER: u8 = CTC_MODE_BIT;
pub const CTC_MODE_TIMER: u8 = 0;

pub const CTC_PRESCALER_BIT: u8 = 1 << 5;
pub const CTC_PRESCALER_256: u8 = CTC_PRESCALER_BIT;
pub const CTC_PRESCALER_16: u8 = 0;

pub const CTC_EDGE_BIT: u8 = 1 << 4;
pub const CTC_EDGE_RISING: u8 = CTC_EDGE_BIT;
pub const CTC_EDGE_FALLING: u8 = 0;

pub const CTC_TRIGGER_BIT: u8 = 1 << 3;
pub const CTC_TRIGGER_PULSE: u8 = CTC_TRIGGER_BIT;
pub const CTC_TRIGGER_AUTOMATIC: u8 = 0;

pub const CTC_CONSTANT_FOLLOWS: u8 = 1 << 2;
pub const CTC_RESET: u8 = 1 << 1;

pub const CTC_CONTROL_BIT: u8 = 1 << 0;
pub const CTC_CONTROL_WORD: u8 = CTC_CONTROL_BIT;
pub const CTC_CONTROL_VECTOR: u8 = 0;

#[derive(Clone,Copy)]
struct Channel {
    pub control: u8,
    pub constant: u8,
    pub down_counter: RegT,
    pub waiting_for_trigger: bool,
    pub int_vector: u8,
}

/// Z80 CTC emulation
pub struct CTC {
    id: usize, // a CTC ID for systems with multiple CTCs
    chn: [Channel; NUM_CHANNELS],
}

impl CTC {
    /// initialize new CTC object
    pub fn new(id: usize) -> CTC {
        CTC {
            id: id,
            chn: [Channel {
                control: CTC_RESET,
                constant: 0,
                down_counter: 0,
                waiting_for_trigger: false,
                int_vector: 0,
            }; NUM_CHANNELS],
        }
    }

    /// reset the CTC
    pub fn reset(&mut self) {
        for chn in &mut self.chn {
            chn.control = CTC_RESET;
            chn.constant = 0;
            chn.down_counter = 0;
            chn.waiting_for_trigger = false;
        }
    }

    /// write a CTC control register
    pub fn write(&mut self, bus: &Bus, chn: usize, val: RegT) {
        let mut notify_bus = false;
        let ctrl = self.chn[chn].control;
        if (ctrl & CTC_CONSTANT_FOLLOWS) == CTC_CONSTANT_FOLLOWS {
            // val is time constant value following a control word
            let c = &mut self.chn[chn];
            c.constant = val as u8;
            c.down_counter = CTC::down_counter_initial(c);
            if (ctrl & CTC_MODE_BIT) == CTC_MODE_TIMER {
                c.waiting_for_trigger = (ctrl & CTC_TRIGGER_BIT) == CTC_TRIGGER_PULSE;
            }
            c.control &= !(CTC_CONSTANT_FOLLOWS | CTC_RESET);
            notify_bus = true;
        } else if (ctrl & CTC_CONTROL_BIT) == CTC_CONTROL_WORD {
            // val is a control word
            let c = &mut self.chn[chn];
            c.control = val as u8;
            if (c.control & CTC_CONSTANT_FOLLOWS) == 0 {
                notify_bus = true;
            }
        } else if chn == CTC_0 {
            // val is interrupt vector for CTC_0, the interrupt vector
            // for the other channels are computed from this
            for i in 0..NUM_CHANNELS {
                self.chn[i].int_vector = ((val & 0xF8) + 2 * i as i32) as u8;
            }
        }

        // notify the system bus if necessary
        if notify_bus {
            bus.ctc_write(chn, self);
        }
    }

    /// read CTC value
    pub fn read(&self, chn: usize) -> RegT {
        let c = self.chn[chn];
        let mut val = c.down_counter as RegT;
        if (c.control & CTC_MODE_BIT) == CTC_MODE_TIMER {
            val /= CTC::prescale(c.control);
        }
        val
    }

    /// update the CTC channel timers
    pub fn update_timers(&mut self, bus: &Bus, cycles: i64) {
        for chn in 0..NUM_CHANNELS {
            let ctrl = self.chn[chn].control;
            let waiting = self.chn[chn].waiting_for_trigger;
            if (ctrl & (CTC_RESET | CTC_CONSTANT_FOLLOWS)) == 0 {
                if (ctrl & CTC_MODE_BIT) == CTC_MODE_TIMER && !waiting {
                    self.chn[chn].down_counter -= cycles as RegT;
                    while self.chn[chn].down_counter <= 0 {
                        self.down_counter_trigger(bus, chn);
                        self.chn[chn].down_counter += CTC::down_counter_initial(&self.chn[chn]);
                    }
                }
            }
        }
    }

    /// get prescaler value (256 or 16) based on prescaler bit
    fn prescale(ctrl: u8) -> RegT {
        if (ctrl & CTC_PRESCALER_BIT) == CTC_PRESCALER_256 {
            256
        } else {
            16
        }
    }

    /// compute intitial down-counter value
    fn down_counter_initial(c: &Channel) -> RegT {
        let mut val: RegT = if 0 == c.constant {
            0x100
        } else {
            c.constant as RegT
        };
        if (c.control & CTC_MODE_BIT) == CTC_MODE_TIMER {
            val *= CTC::prescale(c.control);
        }
        val
    }

    /// trigger interrupt and/or callback when downcounter reaches 0
    fn down_counter_trigger(&self, bus: &Bus, chn: usize) {
        if (self.chn[chn].control & CTC_INTERRUPT_BIT) == CTC_INTERRUPT_ENABLED {
            bus.ctc_irq(self.id, chn, self.chn[chn].int_vector as RegT);
        }
        bus.ctc_zero(chn, self);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn reset() {
        let mut ctc = CTC::new(0);
        ctc.chn[CTC_0].control = CTC_MODE_COUNTER | CTC_PRESCALER_256;
        ctc.chn[CTC_0].constant = 0x40;
        ctc.chn[CTC_0].int_vector = 0xE0;
        ctc.chn[CTC_2].control = CTC_EDGE_RISING | CTC_PRESCALER_16;
        ctc.reset();
        assert!(ctc.chn[CTC_0].control == CTC_RESET);
        assert!(ctc.chn[CTC_0].constant == 0);
        assert!(ctc.chn[CTC_0].int_vector == 0xE0);
        assert!(ctc.chn[CTC_2].control == CTC_RESET);
    }
}
