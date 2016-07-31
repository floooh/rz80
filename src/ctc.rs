#![allow(unused)]
use RegT;
use bus::Bus;

/// CTC channel 0
pub const CTC_0: usize = 0;
/// CTC channel 1 
pub const CTC_1: usize = 1;
/// CTC channel 2
pub const CTC_2: usize = 2;
/// CTC channel 3
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
        let old_ctrl = self.chn[chn].control;
        let new_ctrl = val as u8;
        if (old_ctrl & CTC_CONSTANT_FOLLOWS) == CTC_CONSTANT_FOLLOWS {
            // val is time constant value following a control word
            let c = &mut self.chn[chn];
            c.constant = val as u8;
            c.down_counter = CTC::down_counter_initial(c);
            if (old_ctrl & CTC_MODE_BIT) == CTC_MODE_TIMER {
                c.waiting_for_trigger = (old_ctrl & CTC_TRIGGER_BIT) == CTC_TRIGGER_PULSE;
            }
            c.control &= !(CTC_CONSTANT_FOLLOWS | CTC_RESET);
            notify_bus = true;
        } else if (new_ctrl & CTC_CONTROL_BIT) == CTC_CONTROL_WORD {
            // val is a control word
            let c = &mut self.chn[chn];
            c.control = new_ctrl;
            if (new_ctrl & CTC_CONSTANT_FOLLOWS) == 0 {
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

    /// read current counter or timer value 
    pub fn read(&self, chn: usize) -> RegT {
        let c = self.chn[chn];
        let mut val = c.down_counter as RegT;
        if (c.control & CTC_MODE_BIT) == CTC_MODE_TIMER {
            val /= CTC::prescale(c.control);
        }
        val
    }

    /// externally provided trigger/pulse signal, updates counters
    pub fn trigger(&mut self, bus: &Bus, chn: usize) {
        let ctrl = self.chn[chn].control;
        if (ctrl & (CTC_RESET | CTC_CONSTANT_FOLLOWS)) == 0 {
            self.chn[chn].down_counter -= 1;
            if 0 == self.chn[chn].down_counter {
                self.down_counter_trigger(bus, chn);
                self.chn[chn].down_counter = CTC::down_counter_initial(&self.chn[chn]);
            }
            self.chn[chn].waiting_for_trigger = false;
        }
    }

    /// update the CTC channel timers
    #[inline(always)]
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
    use std::cell::RefCell;
    use super::*;
    use Bus;
    use RegT;

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

    struct TestState {
        ctc_write_called: bool,
        ctc_zero_called: bool,
        ctc_irq_called: bool,
        ctc_zero_counter: i32,
        ctc_irq_counter: i32,

    } 
    struct TestBus {
        state: RefCell<TestState>,
    }
    impl TestBus {
        pub fn new() -> TestBus {
            TestBus {
                state: RefCell::new(TestState {
                    ctc_write_called: false,
                    ctc_zero_called: false,
                    ctc_irq_called: false,
                    ctc_zero_counter: 0,
                    ctc_irq_counter: 0,
                }),
            }
        }
    }
    impl Bus for TestBus {
        fn ctc_write(&self, chn: usize, ctc: &CTC) {
            let mut state = self.state.borrow_mut();
            state.ctc_write_called = true;
        }
        fn ctc_zero(&self, chn: usize, ctc: &CTC) {
            let mut state = self.state.borrow_mut();
            state.ctc_zero_called = true;
            state.ctc_zero_counter += 1;
        }
        fn ctc_irq(&self, ctc: usize, chn: usize, int_vector: RegT) {
            let mut state = self.state.borrow_mut();
            state.ctc_irq_called = true;
            state.ctc_irq_counter += 1;
        }
    }

    #[test]
    fn write_int_vector() {
        let mut ctc = CTC::new(0);
        let bus = TestBus::new();
        assert!(0 == ctc.chn[CTC_0].int_vector);

        // interrupt vector must be written to CTC_0, any other channel
        // is ignored
        ctc.write(&bus, CTC_1, 0xE0);
        assert!(0 == ctc.chn[CTC_0].int_vector);
        assert!(0 == ctc.chn[CTC_1].int_vector);
        assert!(0 == ctc.chn[CTC_2].int_vector);
        assert!(0 == ctc.chn[CTC_3].int_vector);
        
        // writing int-vector to CTC_0, also automatically fills the other vectors
        ctc.write(&bus, CTC_0, 0xE0);
        assert!(0xE0 == ctc.chn[CTC_0].int_vector);
        assert!(0xE2 == ctc.chn[CTC_1].int_vector);
        assert!(0xE4 == ctc.chn[CTC_2].int_vector);
        assert!(0xE6 == ctc.chn[CTC_3].int_vector);
    }

    #[test]
    fn write_control_word() {
        let mut ctc = CTC::new(0);
        let bus = TestBus::new();
        let ctrl = (CTC_CONTROL_WORD | CTC_INTERRUPT_ENABLED | CTC_MODE_COUNTER | CTC_PRESCALER_256) as RegT;
        ctc.write(&bus, CTC_0, ctrl);
        assert!(ctrl == ctc.chn[CTC_0].control as RegT);
        assert!(CTC_RESET == ctc.chn[CTC_1].control);
        assert!(CTC_RESET == ctc.chn[CTC_2].control);
        assert!(CTC_RESET == ctc.chn[CTC_2].control);
        assert!(bus.state.borrow().ctc_write_called);
    }

    fn ctc_counter_test(with_irq: bool) {
        let mut ctc = CTC::new(0);
        let bus = TestBus::new();
        let ctrl_test = (CTC_CONTROL_WORD |
                         if with_irq {CTC_INTERRUPT_ENABLED} else {CTC_INTERRUPT_DISABLED} | 
                         CTC_MODE_COUNTER | 
                         CTC_PRESCALER_256) as RegT; // NOTE: in counter mode, prescale should be ignored!
        let ctrl = ctrl_test | (CTC_CONSTANT_FOLLOWS as RegT);

        ctc.write(&bus, CTC_0, ctrl);
        ctc.write(&bus, CTC_0, 0x20);       // write constant following control word
        assert!(ctrl_test == ctc.chn[CTC_0].control as RegT);
        assert!(0x20 == ctc.chn[CTC_0].constant);
        assert!(0x20 == ctc.chn[CTC_0].down_counter);
        assert!(0x20 == ctc.read(CTC_0));
        assert!(!ctc.chn[CTC_0].waiting_for_trigger);

        // update timer channels, this should *NOT* update the counters
        for i in 0..256 {
            ctc.update_timers(&bus, 10);
        }
        assert!(bus.state.borrow().ctc_zero_counter == 0);
        assert!(bus.state.borrow().ctc_irq_counter == 0);
        assert!(0x20 == ctc.chn[CTC_0].down_counter);
    
        // now trigger counters, this should update the counter and call the ctc_zero() callback
        for i in 0..0x50 {
            ctc.trigger(&bus, CTC_0);
        }
        assert!(bus.state.borrow().ctc_zero_called);
        assert!(bus.state.borrow().ctc_irq_called == with_irq);
        assert!(bus.state.borrow().ctc_zero_counter == 2);
        assert!(bus.state.borrow().ctc_irq_counter == if with_irq {2} else {0});
        assert!(ctc.chn[CTC_0].down_counter == 0x10);
        assert!(ctc.read(CTC_0) == 0x10);
    }

    #[test]
    fn ctc_counter_no_irq() {
        ctc_counter_test(false);
    }

    #[test]
    fn ctc_counter_with_irq() {
        ctc_counter_test(true);
    }

    fn ctc_timer_test(with_irq: bool) {
        let mut ctc = CTC::new(0);
        let bus = TestBus::new();
        let ctrl_test = (CTC_CONTROL_WORD |
                         if with_irq {CTC_INTERRUPT_ENABLED} else {CTC_INTERRUPT_DISABLED} | 
                         CTC_MODE_TIMER | 
                         CTC_PRESCALER_16) as RegT;
        let ctrl = ctrl_test | (CTC_CONSTANT_FOLLOWS as RegT);

        ctc.write(&bus, CTC_0, ctrl);
        ctc.write(&bus, CTC_0, 0x20);       // write constant following control word
        assert!(ctrl_test == ctc.chn[CTC_0].control as RegT);
        assert!(0x20 == ctc.chn[CTC_0].constant);
        assert!(0x200 == ctc.chn[CTC_0].down_counter);
        assert!(0x20 == ctc.read(CTC_0));
        assert!(!ctc.chn[CTC_0].waiting_for_trigger); // CTC_TRIGGER_PULSE was not set

        // update the timer channels
        for i in 0..0x200 {
            ctc.update_timers(&bus, 2);            
        }
        assert!(bus.state.borrow().ctc_zero_called);
        assert!(bus.state.borrow().ctc_irq_called == with_irq);
        assert!(bus.state.borrow().ctc_zero_counter == 2);
        assert!(bus.state.borrow().ctc_irq_counter == if with_irq {2} else {0});
        assert!(ctc.chn[CTC_0].down_counter == 0x200);
        assert!(ctc.read(CTC_0) == 0x20);
    }

    #[test]
    fn ctc_timer_no_irq() {
        ctc_timer_test(false);
    }

    #[test]
    fn ctc_timer_with_irq() {
        ctc_timer_test(true);
    }
}

