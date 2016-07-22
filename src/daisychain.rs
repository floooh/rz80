#![allow(unused)]
use RegT;
use bus::Bus;

const MAX_CONTROLLERS : usize = 16;

/// a single interrupt controller
#[derive(Clone,Copy)]
struct Controller {
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

/// a daisy-chain of interrupt controllers
pub struct Daisychain {
    num_ctrl: usize,
    ctrl : [Controller; MAX_CONTROLLERS],
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
    pub fn irq(&mut self, bus: &mut Bus, ctrl_id: usize, vec: u8) {
        if self.ctrl[ctrl_id].int_enabled {
            {
                let ctrl = &mut self.ctrl[ctrl_id];
                ctrl.int_enabled = false;
                ctrl.int_requested = true;
                ctrl.int_vec = vec;
            }
            bus.irq_cpu();

            // disable interrupt on downstream controllers
            for ctrl in &mut self.ctrl[ctrl_id+1..self.num_ctrl] {
                ctrl.int_enabled = false;
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


