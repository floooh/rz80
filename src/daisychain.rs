#![allow(unused)]
use RegT;

const MAX_CONTROLLERS : usize = 16;

/// a single interrupt controller
#[derive(Clone,Copy)]
struct Controller {
    pub int_enabled : bool,
    pub int_requested : bool,
    pub int_pending : bool,
    pub int_data : u8,
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
            int_enabled: true,
            int_requested: false,
            int_pending: false,
            int_data: 0,
        }
    }
    pub fn reset(&mut self) {
        self.int_enabled = true;
        self.int_requested = false;
        self.int_pending = false;
        self.int_data = 0;
    }
}

/// a daisy-chain of interrupt controllers
pub struct Daisychain {
    ctrl : [Controller; MAX_CONTROLLERS],
}

impl Daisychain {
    pub fn new() -> Daisychain {
        Daisychain {
            ctrl: [Controller::new(); MAX_CONTROLLERS],
        }
    }
}


