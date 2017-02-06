#![allow(unused)]
extern crate rz80;
extern crate time;
extern crate minifb;
extern crate rand;

use rz80::{CPU,PIO,CTC,Daisychain,Bus,RegT,PIO_A,PIO_B,CTC_0,CTC_1,CTC_2,CTC_3};
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
        while cur_cycles < num_cycles {
            let op_cycles = self.cpu.borrow_mut().step(self);
            self.ctc.borrow_mut().update_timers(self, op_cycles);
            cur_cycles += op_cycles;
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

    fn cpu_outp(&self, port: RegT, val: RegT) {
        println!("cpu_outp: port={:x} val={:x}", port & 0xFF, val);
        match port & 0xFF {
            0x80|0x84 => self.ctc.borrow_mut().write(self, CTC_0, val),
            0x81|0x85 => self.ctc.borrow_mut().write(self, CTC_1, val),
            0x82|0x86 => self.ctc.borrow_mut().write(self, CTC_2, val),
            0x83|0x87 => self.ctc.borrow_mut().write(self, CTC_3, val),
            0x88|0x8C => self.pio1.borrow_mut().write_data(self, PIO_A, val),
            0x89|0x8D => self.pio1.borrow_mut().write_data(self, PIO_B, val),
            0x8A|0x8E => self.pio1.borrow_mut().write_control(PIO_A, val),
            0x8B|0x8F => self.pio1.borrow_mut().write_control(PIO_B, val),
            0x90|0x94 => self.pio2.borrow_mut().write_data(self, PIO_A, val),
            0x91|0x95 => self.pio2.borrow_mut().write_data(self, PIO_B, val),
            0x92|0x96 => self.pio2.borrow_mut().write_control(PIO_A, val),
            0x93|0x97 => self.pio2.borrow_mut().write_control(PIO_B, val),
            _ => (),
            
        }
    }

    fn cpu_inp(&self, port: RegT) -> RegT {
        println!("cpu_inp: port={:x}", port & 0xFF);
        match port & 0xFF {
            0x80|0x84 => self.ctc.borrow().read(CTC_0),
            0x81|0x85 => self.ctc.borrow().read(CTC_1),
            0x82|0x86 => self.ctc.borrow().read(CTC_2),
            0x83|0x87 => self.ctc.borrow().read(CTC_3),
            0x88|0x8C => self.pio1.borrow_mut().read_data(self, PIO_A),
            0x89|0x8D => self.pio1.borrow_mut().read_data(self, PIO_B),
            0x8A|0x8E|0x8B|0x8F => self.pio1.borrow().read_control(),
            0x90|0x94 => self.pio2.borrow_mut().read_data(self, PIO_A),
            0x91|0x95 => self.pio2.borrow_mut().read_data(self, PIO_B),
            0x92|0x96|0x93|0x97 => self.pio2.borrow().read_control(),
            _ => 0xFF,
        }
    }

    fn irq(&self, ctrl_id: usize, vec: u8) {
        println!("irq: ctrl_id={:x} vec={:x}", ctrl_id, vec);
    }
    fn irq_cpu(&self) {
        println!("irq_cpu")
    }
    fn irq_ack(&self) -> RegT {
        println!("irq_ack");
        0
    }
    fn irq_reti(&self) {
        println!("irq_reti");
    }

    fn pio_outp(&self, pio: usize, chn: usize, data: RegT) {
        println!("pio_outp: pio={:x} chn={:x} data={:x}", pio, chn, data);
    }
    fn pio_inp(&self, pio: usize, chn: usize) -> RegT {
        println!("pio_in: pio={:x} chn={:x}", pio, chn);
        0
    }
    fn pio_rdy(&self, pio: usize, chn: usize, rdy: bool) {
        println!("pio_rdy: pio={:x} chn={:x} rdy={:}", pio, chn, rdy);
    }
    fn pio_irq(&self, pio: usize, chn: usize, int_vector: RegT) {
        println!("pio_irq: pio={:x} chn={:x} int_vector{:x}", pio, chn, int_vector);
    }

    fn ctc_write(&self, chn: usize, ctc: &CTC) {
        println!("ctc_write: chn={:x}", chn);
    }
    fn ctc_zero(&self, chn: usize, ctc: &CTC) {
        // blargh, and here we are stuck... CTC2 output trigger is connected
        // CTC3 input trigger, and here the snake baits its tail...
        // ...back to the drawing board...
        println!("ctc_zero: chn={:x}", chn);
    }
    fn ctc_irq(&self, ctc: usize, chn: usize, int_vector: RegT) {
        println!("ctc_irq: ctc={:x}, chn={:x}, int_vector={:x}", ctc, chn, int_vector);
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
    let mut frame_buffer = vec![0u32; WIDTH*HEIGHT];

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

