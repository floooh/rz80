#![allow(unused)]
use std::cell::RefCell;
use RegT;
use bus::Bus;

const MAX_CONTROLLERS : usize = 16;

/// a single interrupt controller
#[derive(Clone,Copy)]
pub struct Controller {
    pub int_enabled : bool,
    pub int_requested : bool,
    pub int_pending : bool,
    pub int_vec : u8,
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
            int_enabled: true,
            int_requested: false,
            int_pending: false,
            int_vec: 0,
        }
    }
    pub fn reset(&mut self) {
        self.int_enabled = true;
        self.int_requested = false;
        self.int_pending = false;
        self.int_vec = 0;
    }
}

/// interrupt controller daisychain
pub struct Daisychain {
    pub num_ctrl: usize,
    pub ctrl : [Controller; MAX_CONTROLLERS],
}

impl Daisychain {
    /// initialize a new daisychain
    pub fn new(num_controllers: usize) -> Daisychain {
        Daisychain {
            num_ctrl: num_controllers,
            ctrl: [Controller::new(); MAX_CONTROLLERS],
        }
    }

    /// reset interrupt controllers in daisychain
    pub fn reset(&mut self) {
        for ctrl in self.ctrl.iter_mut() {
            ctrl.reset();
        }
    }

    /// request an interrupt from an interrupt controller, called by bus
    pub fn irq(&mut self, bus: &Bus, ctrl_id: usize, vec: u8) {
        if self.ctrl[ctrl_id].int_enabled {
            {
                let ctrl = &mut self.ctrl[ctrl_id];
                assert!(!ctrl.int_pending);
                ctrl.int_enabled = false;
                ctrl.int_requested = true;
                ctrl.int_vec = vec;
            }
            bus.irq_cpu();

            // disable interrupt on downstream controllers
            for i in ctrl_id+1..self.num_ctrl {
                self.ctrl[i].int_enabled = false;
            }
        }
    }

    /// CPU acknowledges interrupt request, return the interrupt vector
    pub fn irq_ack(&mut self) -> RegT {
        // find the interrupt controller which issued the request
        // and return it's interrupt vector.
        // downstream controller remain in interrupt-disabled
        // state until the CPU sends the RETI
        for ctrl in self.ctrl.iter_mut() {
            if ctrl.int_requested {
                ctrl.int_requested = false;
                ctrl.int_pending = true;
                return ctrl.int_vec as RegT;
            }
        }
        panic!("irq_ack() called without any interrupt pending!")
    }

    /// CPU executes a RETI, this enabled interrupts on downstream controllers
    pub fn irq_reti(&mut self) {
        let mut is_downstream = false;
        for ctrl in self.ctrl.iter_mut() {
            ctrl.int_enabled = true;
            if ctrl.int_pending {
                if is_downstream {
                    // interrupt-enable propagation stops at
                    // first downstream device where an interrupt
                    // is still pending
                    break;
                }
                ctrl.int_pending = false;
                is_downstream = true;
            }
        }
    }
}

//------------------------------------------------------------------------------
#[cfg(test)]
mod test {
    use std::cell::RefCell;
    use super::*;
    use RegT;
    use Bus;
    use CPU;

    #[test]
    fn reset() {
        let mut daisy = Daisychain::new(8);
        assert!(8 == daisy.num_ctrl);
        for ctrl in daisy.ctrl.iter() {
            assert!(ctrl.int_enabled);
            assert!(!ctrl.int_requested);
            assert!(!ctrl.int_pending);
            assert!(0 == ctrl.int_vec);
        }
        daisy.ctrl[0].int_enabled = false;
        daisy.ctrl[1].int_requested = true;
        daisy.ctrl[2].int_pending = true;
        daisy.ctrl[3].int_vec = 0x10;
        daisy.reset();
        for ctrl in daisy.ctrl.iter() {
            assert!(ctrl.int_enabled);
            assert!(!ctrl.int_requested);
            assert!(!ctrl.int_pending);
            assert!(0 == ctrl.int_vec);
        }
    }

    const DEV0: usize = 0;
    const DEV1: usize = 1;
    const DEV2: usize = 2;
    const NUM_DEVS: usize = 3;

    struct State {
        pub irq_received: bool,
        pub irq_ctrl_id: usize,
        pub irq_vec: u8,
        pub irq_cpu_called: bool,
    }
    struct TestBus {
        pub state: RefCell<State>,
        pub daisy: RefCell<Daisychain>,
        pub cpu: RefCell<CPU>,
    }
    impl TestBus {
        pub fn new() -> TestBus {
            TestBus {
                state: RefCell::new(State {
                    irq_received: false,
                    irq_ctrl_id: 0xFF,
                    irq_vec: 0,
                    irq_cpu_called: false,
                }),
                daisy: RefCell::new(Daisychain::new(NUM_DEVS)),
                cpu: RefCell::new(CPU::new()),
            }
        }
    }

    impl Bus for TestBus {
        fn irq(&self, ctrl_id: usize, vec: u8) {
            let mut state = self.state.borrow_mut();
            state.irq_received = true;
            state.irq_ctrl_id = ctrl_id;
            state.irq_vec = vec;
        }
        fn irq_cpu(&self) { 
            let mut state = self.state.borrow_mut();
            state.irq_cpu_called = true;
        }
    }

    #[test]
    fn irq_ack() {
        let bus = TestBus::new();
        let mut daisy = bus.daisy.borrow_mut();
        // test with interrupt disabled
        daisy.ctrl[DEV0].int_enabled = false;
        daisy.irq(&bus, DEV0, 0x10);
        {
            let dev0 = &daisy.ctrl[DEV0];
            let state = bus.state.borrow();
            assert!(!dev0.int_enabled);
            assert!(!dev0.int_requested);
            assert!(!dev0.int_pending);
            assert!(dev0.int_vec == 0x00);
            assert!(!state.irq_received);
        }
        // test with interrupt enabled
        daisy.ctrl[DEV0].int_enabled = true;
        daisy.irq(&bus, DEV0, 0x10);
        {
            let dev0 = &daisy.ctrl[DEV0];
            let dev1 = &daisy.ctrl[DEV1];
            let dev2 = &daisy.ctrl[DEV2];
            let state = bus.state.borrow();
            assert!(!dev0.int_enabled);
            assert!(dev0.int_requested);
            assert!(!dev0.int_pending);
            assert!(dev0.int_vec == 0x10);
            assert!(state.irq_cpu_called);
            assert!(!dev1.int_enabled);
            assert!(!dev2.int_enabled);
        }
    }
}


