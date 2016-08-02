#![allow(unused)]
extern crate rz80;
extern crate time;
extern crate minifb;
extern crate rand;

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
const FREQ_KHZ: i64 = 2458;

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

impl System {
    pub fn new() -> System {
        System {
            cpu: RefCell::new(CPU::new()),
            pio1: RefCell::new(PIO::new(0)),
            pio2: RefCell::new(PIO::new(1)),
            ctc: RefCell::new(CTC::new(0)),
            daisy: RefCell::new(Daisychain::new(8))
        }
    }

    pub fn poweron(&mut self) {
        let mut cpu = self.cpu.borrow_mut();
        
        // map 48 KByte RAM
        cpu.mem.map(0, 0x00000, 0x0000, true, 0xC000);
        // 2 KByte video RAM (1 KByte colors, 1 KByte ASCII)
        cpu.mem.map(0, 0x0E800, 0xE800, true, 0x0800);

        // BASIC and OS ROMs
        cpu.mem.map_bytes(1, 0x10000, 0xC000, false, &BASIC);
        cpu.mem.map_bytes(1, 0x12000, 0xE000, false, &OS);

        // fill video and color RAM with randomness
        for b in &mut cpu.mem.heap[0x0E800..0xF000] {
            *b = rand::random();
        }

        // set PC to ROM start
        cpu.reg.set_pc(0xF000);
    }
    
    // run the emulator for one frame
    pub fn step_frame(&self, micro_seconds: i64) {
        let num_cycles = (FREQ_KHZ * micro_seconds) / 1000;
        let mut cur_cycles = 0;
        let mut cpu = self.cpu.borrow_mut();
        while cur_cycles < num_cycles {
            cur_cycles += cpu.step(self);
        }
    }

    #[inline(always)]
    fn rgba8(color: u8) -> u32 {
        match color {
            0 => 0xFF000000,        // black
            1 => 0xFFFF0000,        // red
            2 => 0xFF00FF00,        // green
            3 => 0xFFFFFF00,        // yellow
            4 => 0xFF0000FF,        // blue
            5 => 0xFFFF00FF,        // purple
            6 => 0xFF00FFFF,        // cyan
            _ => 0xFFFFFFFF,        // white
        }
    }

    pub fn decode_framebuffer(&self, fb: &mut [u32]) {
        let mut fb_iter = fb.iter_mut();
        let cpu = self.cpu.borrow();
        let blinking = true;   // FIXME
        let video_mem = &cpu.mem.heap[0xEC00..0xF000];
        let color_mem = &cpu.mem.heap[0xE800..0xEC00];
        let mut off = 0;
        for y in 0..24 {
            for py in 0..8 {
                for x in 0..40 {
                    let chr = video_mem[off+x] as usize;
                    let bits = FONT[(chr<<3)|py];
                    let color = color_mem[off+x];
                    let b = (color & 0x80) != 0 && blinking;
                    let fg_bits = if b {color & 7} else {(color>>4) & 7};
                    let bg_bits = if b {(color>>4) & 7} else {color & 7};
                    let fg = System::rgba8(fg_bits);
                    let bg = System::rgba8(bg_bits);
                    for px in 0..8 {
                        let pixel = if (bits & (0x80>>px)) != 0 {fg} else {bg};
                        *fb_iter.next().unwrap() = pixel;
                    }
                }
            }
            off += 40;
        }
    }
}

impl Bus for System {
    fn pio_outp(&self, _: usize, chn: usize, data: RegT) {
        println!("pio_outp called!");
    }
    fn pio_inp(&self, _: usize, chn: usize) -> RegT {
        println!("pio_inp called!");
        0
    }
    
}

fn main() {
    // create a window via minifb
    let mut window = match Window::new("rz80 KC87 example (WIP)",
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

    let mut system = System::new();
    system.poweron();
    let mut micro_seconds_per_frame: i64 = 0;
    while window.is_open() {
        let start = PreciseTime::now();

        // run the emulator for the current frame
        system.step_frame(micro_seconds_per_frame);

        // update the window content
        system.decode_framebuffer(&mut frame_buffer);
        window.update_with_buffer(&frame_buffer); 

        // measure the elapsed time to run emulator at the correct speed
        let frame_time = start.to(PreciseTime::now());
        micro_seconds_per_frame = frame_time.num_microseconds().unwrap();
    }
}

