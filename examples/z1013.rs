#![allow(unused)]
extern crate rz80;
extern crate time;
extern crate minifb;

use std::cell::RefCell;
use minifb::{Window, Key, Scale, WindowOptions};
use rz80::*;

const FB_WIDTH: usize=256;          // Z1013 framebuffer width
const FB_HEIGHT: usize=256;         // Z1013 framebuffer height
const Z1013_MAX_KEYS: usize=128;
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

    pub fn put_key(&mut self, ascii: u8) {
        let key_index = (ascii as usize) & (Z1013_MAX_KEYS-1);
        match ascii {
            0 => { 
                self.next_kbd_column_bits = 0; 
            },
            _ => { 
                self.next_kbd_column_bits = self.key_map[key_index]; 
            }
        }
    }
}

struct System {
    pub cpu: RefCell<CPU>,
    pub pio: RefCell<PIO>,
    pub daisy: RefCell<Daisychain>,
    pub z1013: RefCell<Z1013>,
}

impl Bus for System {

    fn cpu_outp(&self, port: RegT, val: RegT) {
        match port & 0xFF {
            // PIO-A data
            0x00 => {
                // println!("PIO-A write data {:x}", val);
                self.pio.borrow_mut().write_data(self, PIO_A, val);
            },
            // PIO-A, control
            0x01 => {
                // println!("PIO-A write control {:x}", val);
                self.pio.borrow_mut().write_data(self, PIO_A, val);
            },
            // PIO-B, data
            0x02 => {
                // println!("PIO-B write data {:x}", val);
                self.pio.borrow_mut().write_data(self, PIO_B, val);
            },
            // PIO-B, data
            0x03 => {
                // println!("PIO-B write control {:x}", val);
                self.pio.borrow_mut().write_data(self, PIO_B, val);
            },
            // keyboard column
            0x08 => {
                // println!("keyboard column {:x}", val);
                let mut z1013 = self.z1013.borrow_mut();
                if val == 0 {
                    z1013.kbd_column_bits = z1013.next_kbd_column_bits;
                }
                z1013.kbd_column_nr_requested = val as usize;
            },
            _ => {
                println!("CPU OUT: {:x}", val);
            }
        }
    }
    
    fn cpu_inp(&self, port: RegT) -> RegT {
        match port & 0xFF {
            // PIO-A data
            0x00 => {
                // println!("PIO-A read data");
                self.pio.borrow_mut().read_data(self, PIO_A)
            },
            // PIO-A control
            0x01 => {
                // println!("PIO-A read control");
                self.pio.borrow_mut().read_control()
            },
            // PIO-B read data
            0x02 => {
                // println!("PIO-B read data");
                self.pio.borrow_mut().read_data(self, PIO_B)
            },
            // PIO-B read control
            0x03 => {
                // println!("PIO-B read control");
                self.pio.borrow_mut().read_control()
            },
            _ => {
                println!("CPU IN: {:x}", port);
                0xFF
            }
        }
    }

    fn pio_outp(&self, pio: usize, chn: usize, data: RegT) {
        if chn == PIO_B {
            // println!("PIO-B out {:x}", data);
            let mut z1013 = self.z1013.borrow_mut();
            z1013.kbd_8x8_requested = 0 != (data & (1<<4));
        }
        else {
            println!("PIO-A output (unused)");
        }
    }

    fn pio_inp(&self, pio: usize, chn: usize) -> RegT {
        if chn == PIO_B {
            // println!("PIO-B in");
            let z1013 = self.z1013.borrow();
            let col = z1013.kbd_column_nr_requested & 7;
            let mut val = z1013.kbd_column_bits >> (col*8);
            if z1013.kbd_8x8_requested {
                val >>= 4;
            }
            val = 0xF & !(val & 0xF);
            val as RegT
        }
        else {
            println!("PIO-A input (unused)");
            0xFF
        }
    }

    fn irq(&self, ctrl_id: usize, vec: u8) { 
        println!("IRQ");
    }
    fn irq_cpu(&self) {
        println!("IRQ CPU");
    }

    fn irq_ack(&self) -> RegT { 
        println!("IRQ ACK");
        0 
    }
    fn irq_reti(&self) { 
        println!("IRQ RETI");
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
                    for px in 0..8 {
                        fb[i] = if (bits & (1<<(7-px))) != 0 {0xFFFFFFFF} else {0xFF000000};
                        i += 1;
                    }
                }
            }
        }
    }

    pub fn put_key(&mut self, ascii: u8) {
        let mut z1013 = self.z1013.borrow_mut();
        z1013.put_key(ascii);
    }
}

fn create_window() -> Window {
    match Window::new("rz80 Z1013 Example",
               FB_WIDTH, FB_HEIGHT,
               WindowOptions {
                   resize: false,
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
        if window.is_key_down(Key::A) {
            println!("KEY!");
            system.put_key(b'A');
        }
        else {
            system.put_key(0);
        }
        system.step(micro_seconds_per_frame);
        system.decode_video(&mut frame_buffer);
        window.update_with_buffer(&frame_buffer); 
    }
}

