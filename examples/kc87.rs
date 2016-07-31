#![allow(unused)]
extern crate rz80;
extern crate time;
extern crate minifb;

use rz80::{CPU,PIO,CTC,Daisychain,Bus,RegT,PIO_A,PIO_B};
use minifb::{Key, Window, Scale, WindowOptions};
use time::PreciseTime;
use std::cell::RefCell;

// binary dumps for OS, font and BASIC interpreter
static OS: &'static [u8] = include_bytes!("dumps/kc87_os_2.bin");
static FONT: &'static [u8] = include_bytes!("dumps/kc87_font_2.bin");
static BASIC: &'static [u8] = include_bytes!("dumps/z9001_basic.bin");

// framebuffer dimensions (40x24 characters at 8x8 pixels)
const WIDTH: usize = 320;
const HEIGHT: usize = 192;
// number of keys in key mapping tables
const MAX_KEYS: usize = 128;
// CPU frequency in kHZ
const FREQ_KHZ: usize = 2458;

struct KC87 {
    key_mask: u64,
    kbd_column_mask: u8,
    kbd_line_mask: u8,
    key_map: [u64; MAX_KEYS],
    blink_flip_flop: bool,
}

struct System {
    pub cpu: RefCell<CPU>,
    pub pio1: RefCell<PIO>,
    pub pio2: RefCell<PIO>,
    pub ctc: RefCell<CTC>,
    pub daisy: RefCell<Daisychain>,
}

fn main() {
    // create a window via minifb
    let mut window = match Window::new("rz80 KC87 Example",
           WIDTH, HEIGHT,
           WindowOptions {
               resize: false,
               scale: Scale::X2,
               ..WindowOptions::default()
           }) {
        Ok(win) => win,
        Err(err) => panic!("Unable to create minifb window: {}", err)
    };

    // the pixel frame buffer, written by System::decode_framebuffer()
    // and transfered to the minifb window
    let mut frame_buffer = [0u32; WIDTH*HEIGHT];

    while window.is_open() {
        window.update_with_buffer(&frame_buffer);
    }   
}

