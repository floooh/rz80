#![allow(unused)]
extern crate rz80;
extern crate time;
extern crate minifb;

use std::cell::RefCell;
use minifb::{Window, Key, Scale, WindowOptions};
use rz80::{RegT,Bus,CPU,PIO,Daisychain};

const FB_WIDTH: usize=256;          // Z1013 framebuffer width
const FB_HEIGHT: usize=256;         // Z1013 framebuffer height
const Z1013_MAX_KEYS: usize=128;    // 
const Z1013_FREQ_KHZ: i64=2000;     // frequency (2MHz = 2000 kHz)

const DEV_PIO: usize=0;
const DEV_PIO_A: usize=0;
const DEV_PIO_B: usize=1;
const NUM_DEVS: usize=2;

static Z1013_MON_A2: &'static [u8] = include_bytes!("z1013_mon_a2.bin");
static Z1013_FONT:   &'static [u8] = include_bytes!("z1013_font.bin");

struct Z1013 {
    kbd_column_nr_requested: usize,
    kbd_8x8_requested: bool,
    next_kbd_column_bits: u64,
    kbd_column_bits: u64,
    key_map: [u64; Z1013_MAX_KEYS],
}

impl Z1013 {
    pub fn new() -> Z1013 {
        let mut z1013 = Z1013 {
            kbd_column_nr_requested: 0,
            kbd_8x8_requested: false,
            next_kbd_column_bits: 0,
            kbd_column_bits: 0,
            key_map: [0; Z1013_MAX_KEYS],
        };
        z1013.init_keymap_8x8();
        z1013
    }

    fn kbd_bit(col: usize, line: usize, num_lines: usize) -> u64 {
        (1u64<<line)<<(col*num_lines)
    }

    fn init_key(&mut self, ascii: u8, col: usize, line: usize, shift:usize, num_lines: usize) {
        let mask = 
            Z1013::kbd_bit(col, line, num_lines) |
            if shift != 0 {Z1013::kbd_bit(7, 6, num_lines)} else {0};
        self.key_map[ascii as usize] = mask; 
    }

    fn init_keymap_8x8(&mut self) {

        // 8x8 keyboard matrix with 2 shift-layers
        let layers_8x8 =
            b"13579-  \
              QETUO@  \
              ADGJL*  \
              YCBM.^  \
              24680[  \
              WRZIP]  \
              SFHK+\\  \
              XVN,/_  \
              !#%')=  \
              qetuo`  \
              adgjl:  \
              ycbm>~  \
              \"$&( {  \
              wrzip}  \
              sfhk;|  \
              xvn<?   ";
        for shift in 0..2 {
            for line in 0..8 {
                for col in 0..8 {
                    let c = layers_8x8[shift*64 + line*8 + col];
                    if 0x20 != c {
                        self.init_key(c, col, line, shift, 8);
                    }
                }
            }
        }

        // special keys
        self.init_key(0x20, 6, 4, 0, 8);    // space
        self.init_key(0x08, 6, 2, 0, 8);    // cursor left
        self.init_key(0x09, 6, 3, 0, 8);    // cursor right
        self.init_key(0x0A, 6, 7, 0, 8);    // cursor down
        self.init_key(0x0B, 6, 6, 0, 8);    // cursor up
        self.init_key(0x0D, 6, 1, 0, 8);    // enter

        // Ctrl+C (== STOP/BREAK)
        self.key_map[0x03] = Z1013::kbd_bit(6, 5, 8) | Z1013::kbd_bit(1, 3, 4);
    }

    pub fn poweron(&mut self, cpu: &mut CPU) {
        
        // reset the hardware
        self.init_keymap_8x8();
        self.kbd_column_nr_requested = 0;
        self.kbd_8x8_requested = false;
        self.next_kbd_column_bits = 0;
        self.kbd_column_bits = 0;

        // map memory (64 KByte RAM incl. vid-mem, and 2 KByte ROM)
        cpu.mem.unmap_all();
        cpu.mem.map(1, 0x00000, 0x0000, true, 1<<16);
        cpu.mem.map_bytes(0, 0x10000, 0xF000, false, &Z1013_MON_A2);

        // start execution at address 0xF000
        cpu.reg.set_pc(0xF000);
    }

    pub fn reset(&mut self, cpu: &mut CPU) {
        self.kbd_column_nr_requested = 0;
        self.next_kbd_column_bits = 0;
        self.kbd_column_bits = 0;
        cpu.reg.set_pc(0xF000);
    }
}

struct System {
    pub cpu: RefCell<CPU>,
    pub pio: RefCell<PIO>,
    pub daisy: RefCell<Daisychain>,
    pub z1013: RefCell<Z1013>,
}

impl Bus for System {
    fn cpu_inp(&self, port: RegT) -> RegT {
        println!("IN {}", port);
        0
    }
    fn cpu_outp(&self, port: RegT, val: RegT) {
        println!("OUT {},{}", port, val);
    }
}

impl System {
    pub fn new() -> System {
        System {
            cpu: RefCell::new(CPU::new()),
            pio: RefCell::new(PIO::new(DEV_PIO)),
            daisy: RefCell::new(Daisychain::new(NUM_DEVS)),
            z1013: RefCell::new(Z1013::new()),
        }
    }

    pub fn poweron(&self) {
        let mut z1013 = self.z1013.borrow_mut();
        let mut cpu = self.cpu.borrow_mut();
        z1013.poweron(&mut cpu);
    }

    pub fn reset(&self) {
        let mut cpu = self.cpu.borrow_mut();
        let mut pio = self.pio.borrow_mut();
        let mut daisy = self.daisy.borrow_mut();
        let mut z1013 = self.z1013.borrow_mut();
        cpu.reset();
        pio.reset();
        daisy.reset();
        z1013.reset(&mut cpu);
    }

    pub fn step(&self, micro_seconds: i64) {
        let num_cycles = (Z1013_FREQ_KHZ * micro_seconds) / 1000;
        let mut cur_cycles = 0;
        let mut cpu = self.cpu.borrow_mut();
        while cur_cycles < num_cycles {
            cur_cycles += cpu.step(self);
        }
    }

    pub fn decode_video(&self, fb: &mut [u32]) {
        let mut i: usize = 0;
        let cpu = self.cpu.borrow();
        let vid_mem = &cpu.mem.heap[0xEC00..0xF000];
        for y in 0..32 {
            for py in 0..8 {
                for x in 0..32 {
                    let chr = vid_mem[(y<<5)+x] as usize;
                    let bits = Z1013_FONT[(chr<<3)|py];
                    fb[i+0] = if (bits & 0x80) != 0 {0xFFFFFFFF} else {0xFF000000};
                    fb[i+1] = if (bits & 0x40) != 0 {0xFFFFFFFF} else {0xFF000000};
                    fb[i+2] = if (bits & 0x20) != 0 {0xFFFFFFFF} else {0xFF000000};
                    fb[i+3] = if (bits & 0x10) != 0 {0xFFFFFFFF} else {0xFF000000};
                    fb[i+4] = if (bits & 0x08) != 0 {0xFFFFFFFF} else {0xFF000000};
                    fb[i+5] = if (bits & 0x04) != 0 {0xFFFFFFFF} else {0xFF000000};
                    fb[i+6] = if (bits & 0x02) != 0 {0xFFFFFFFF} else {0xFF000000};
                    fb[i+7] = if (bits & 0x01) != 0 {0xFFFFFFFF} else {0xFF000000};
                    i += 8;
                }
            }
        }
    }
}

fn create_window() -> Window {
    match Window::new("rustkc85 (Z1013)",
               FB_WIDTH, FB_HEIGHT,
               WindowOptions {
                   resize: true,
                   scale: Scale::X2,
                   ..WindowOptions::default()
               }) {
        Ok(win) => win,
        Err(err) => panic!("Unable to create minifb window: {}", err)
    }
}

fn main() {
    let mut system = System::new();
    let mut window = create_window(); 
    let mut frame_buffer = [0u32; FB_WIDTH*FB_HEIGHT];
    let micro_seconds_per_frame: i64 = 1000000 / 60;
    system.poweron();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        system.step(micro_seconds_per_frame);
        system.decode_video(&mut frame_buffer);
        window.update_with_buffer(&frame_buffer);   
    }
}

