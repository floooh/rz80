///
/// A minimal sample Z1013 emulator.
///
/// The Z1013 is a very simple Z80-based home computer, just a CPU,
/// a PIO, some RAM, ROM and a keyboard matrix. The system emulated
/// here is a Z1013.64:
///
/// - a Z80 CPU running at 2 MHz
/// - a Z80 PIO, channel A is unused, channel B is used for keyboard input
/// - 64 KByte RAM, 1 KByte used as 32x32 ASCII video memory
/// - 2 KByte ROM with a very simple operating system
/// - an 8x8 keyboard matrix
///
/// The sample emulator is *minimal* which means not all features are
/// implemented (e.g. cassette tape in/out, sound output), the Z1013 doesn't
/// require interrupts to function, so all interrupt handling and the
/// interrupt daisy chain is left out.
///
/// For convenience, a BASIC interpreter has been preloaded (this would
/// normally happen by loading from cassette tape). To start the
/// BASIC interpreter, type 'J 300[Enter]' on the command prompt,
/// to write a little Hello World program (NOTE: currently, an 
/// American-English keyboard layout is hardcoded, sorry for the 
/// inconvenience):
///
/// OK
/// >AUTO[Enter]
/// 10 FOR I=0 TO 10[Enter]
/// 20 PRINT "HELLO WORLD"[Enter]
/// 30 NEXT[Enter]
/// 40 [Escape]
/// >RUN[Enter]
/// 
/// To leave the BASIC interpreter, type 'BYE[Enter]'

extern crate rz80;
extern crate time;
extern crate minifb;

use rz80::{CPU,PIO,Bus,RegT,PIO_A,PIO_B};
use minifb::{Key, Window, Scale, WindowOptions};
use time::{PreciseTime};
use std::cell::RefCell;

// binary dump of the Z1013 operatin system ROM, font data and a BASIC interpreter
static Z1013_MON_A2: &'static [u8] = include_bytes!("z1013_mon_a2.bin");
static Z1013_FONT:   &'static [u8] = include_bytes!("z1013_font.bin");
static KC_BASIC:     &'static [u8] = include_bytes!("kc_basic.z80"); 

// required framebuffer dimensions
const FB_WIDTH: usize=256;
const FB_HEIGHT: usize=256;
// max number of entries in key-mapping tables
const Z1013_MAX_KEYS: usize=128;
// CPU frequency in KHz
const Z1013_FREQ_KHZ: i64=2000;

// a mapping of all required minifb key codes to their ASCII values (with and
// without shift key pressed), this only works for english keyboard layouts
static KEYS: &'static [(Key,u8,u8)] = &[
    (Key::Key0,b'0',b')'), (Key::Key1,b'1',b'!'), (Key::Key2,b'2',b'@'), (Key::Key3,b'3',b'#'),
    (Key::Key4,b'4',b'$'), (Key::Key5,b'5',b'%'), (Key::Key6,b'6',b'^'), (Key::Key7,b'7',b'&'),
    (Key::Key8,b'8',b'*'), (Key::Key9,b'9',b'('), (Key::Minus,b'-',b'_'), (Key::Equal,b'=',b'+'),
    (Key::A,b'A',b'a'), (Key::B,b'B',b'b'), (Key::C,b'C',b'c'), (Key::D,b'D',b'd'),
    (Key::E,b'E',b'e'), (Key::F,b'F',b'f'), (Key::G,b'G',b'g'), (Key::H,b'H',b'h'),
    (Key::I,b'I',b'i'), (Key::J,b'J',b'j'), (Key::K,b'K',b'k'), (Key::L,b'L',b'l'),
    (Key::M,b'M',b'm'), (Key::N,b'N',b'n'), (Key::O,b'O',b'o'), (Key::P,b'P',b'p'),
    (Key::Q,b'Q',b'q'), (Key::R,b'R',b'r'), (Key::S,b'S',b's'), (Key::T,b'T',b't'),
    (Key::U,b'U',b'u'), (Key::V,b'V',b'v'), (Key::W,b'W',b'w'), (Key::X,b'X',b'x'),
    (Key::Y,b'Y',b'y'), (Key::Z,b'Z',b'z'),
    (Key::Comma,b',',b'<'), (Key::Period,b'.',b'>'), (Key::Slash,b'/',b'?'),
    (Key::LeftBracket,b'[',b'{'), (Key::RightBracket,b']',b'}'),
    (Key::Semicolon,b';',b':'), (Key::Apostrophe,b'\'',b'"'), (Key::Backslash,b'\\',b'|'),
    (Key::Space,0x20,0x20), (Key::Left,0x08,0x08), (Key::Right,0x09,0x09), (Key::Down,0x0A,0x0A),
    (Key::Up, 0x0B, 0x0B), (Key::Enter,0x0D,0x0D), (Key::Escape, 0x03, 0x03),
];

// The Z1013 8x8 keyboard matrix, in 2 layers (no-shift and shift),
// the array items are the ASCII code of the key at that position
// of the 8x8 keyboard matrix, the first 64 entries if the shift
// key isn't pressed, and the second 64 entries if the shift is pressed.
// Spaces are either unassigned keys, or special keys (like Enter, Escape, ...)
static KEY_MATRIX: &'static [u8] =
    b"13579-  QETUO@  ADGJL*  YCBM.^  24680[  WRZIP]  SFHK+\\  XVN,/_  \
      !#%')=  qetuo`  adgjl:  ycbm>~  \"$&( {  wrzip}  sfhk;|  xvn<?   ";

// The Z1013 struct hold some Z1013 state need in addition to its hardware 
// components, all of this is related to the keyboard input emulation:
//
// Each entry in the key_map array holds the complete state of the 8x8 
// keyboard matrix for each possible ASCII key code.
//
// Keyboard input on the Z1013 works by polling (not interrupt based).
// The CPU will 'light up' columns in the keyboard matrix by 
// sending the column numbers 0..7 to the output port 0x08, 
// ...TO BE CONTINUED...
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
        for shift in 0..2 {
            for line in 0..8 {
                for col in 0..8 {
                    let c = KEY_MATRIX[shift*64 + line*8 + col];
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
        self.key_map[0x03] = Z1013::kbd_bit(6, 5, 8) | Z1013::kbd_bit(1, 3, 8);
    }

    pub fn poweron(&mut self) {
        self.kbd_column_nr_requested = 0;
        self.kbd_8x8_requested = false;
        self.next_kbd_column_bits = 0;
        self.kbd_column_bits = 0;
    }

    pub fn put_key(&mut self, ascii: u8) {
        let key_index = (ascii as usize) & (Z1013_MAX_KEYS-1);
        match ascii {
            0 => self.next_kbd_column_bits = 0,
            _ => self.next_kbd_column_bits = self.key_map[key_index],
        }
    }
}

// The System struct owns all the hardware components
// and implements the Bus trait, which implements the 
// emulator-specific 'wiring'
struct System {
    pub cpu: RefCell<CPU>,
    pub pio: RefCell<PIO>,
    pub z1013: RefCell<Z1013>,
}

// The Bus trait, implemented for the Z1013. This defines how the
// various hardware components in an emulated system talk to each other.
impl Bus for System {

    // cpu_outp() is called when the CPU executes an OUT instruction, on the
    // Z1013 there are 4 important output addresses:
    // 0x00:    PIO-A data (unused)
    // 0x01:    PIO-A control (unused)
    // 0x02:    PIO-B data (keyboard input)
    // 0x03:    PIO-B control (keyboard input)
    // 0x08:    connected to a demultiplexer to light up keyboard matrix columns
    fn cpu_outp(&self, port: RegT, val: RegT) {
        match port & 0xFF {
            0x00 => self.pio.borrow_mut().write_data(self, PIO_A, val),
            0x01 => self.pio.borrow_mut().write_control(PIO_A, val),
            0x02 => self.pio.borrow_mut().write_data(self, PIO_B, val),
            0x03 => self.pio.borrow_mut().write_control(PIO_B, val),
            0x08 => {
                let mut z1013 = self.z1013.borrow_mut();
                if val == 0 {
                    z1013.kbd_column_bits = z1013.next_kbd_column_bits;
                }
                z1013.kbd_column_nr_requested = val as usize;
            },
            _ => ()
        }
    }
    
    // cpu_inp() is called when the CPU executes an IN instruction,
    // it simply reads the PIO data and control registers back
    fn cpu_inp(&self, port: RegT) -> RegT {
        match port & 0xFF {
            0x00 => self.pio.borrow_mut().read_data(self, PIO_A),
            0x01 => self.pio.borrow_mut().read_control(),
            0x02 => self.pio.borrow_mut().read_data(self, PIO_B),
            0x03 => self.pio.borrow_mut().read_control(),
            _ => 0xFF
        }
    }

    fn pio_outp(&self, _: usize, chn: usize, data: RegT) {
        if chn == PIO_B {
            let mut z1013 = self.z1013.borrow_mut();
            z1013.kbd_8x8_requested = 0 != (data & (1<<4));
        }
    }

    fn pio_inp(&self, _: usize, chn: usize) -> RegT {
        if chn == PIO_B {
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
            0xFF
        }
    }
}

impl System {
    pub fn new() -> System {
        System {
            cpu: RefCell::new(CPU::new()),
            pio: RefCell::new(PIO::new(0)),
            z1013: RefCell::new(Z1013::new()),
        }
    }

    pub fn poweron(&self) {
        let mut z1013 = self.z1013.borrow_mut();
        z1013.poweron();
        
        // map memory (64 KByte RAM incl. vid-mem, and 2 KByte ROM)
        let mut cpu = self.cpu.borrow_mut();
        cpu.mem.unmap_all();
        cpu.mem.map(1, 0x00000, 0x0000, true, 1<<16);
        cpu.mem.map_bytes(0, 0x10000, 0xF000, false, &Z1013_MON_A2);

        // copy BASIC interpreter dump into RAM
        cpu.mem.write(0x0100, &KC_BASIC[0x20..]);

        // start execution at address 0xF000
        cpu.reg.set_pc(0xF000);
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

fn main() {
    let mut system = System::new();
    let mut window = match Window::new("rz80 Z1013 Example",
           FB_WIDTH, FB_HEIGHT,
           WindowOptions {
               resize: false,
               scale: Scale::X2,
               ..WindowOptions::default()
           }) {
        Ok(win) => win,
        Err(err) => panic!("Unable to create minifb window: {}", err)
    };
    let mut frame_buffer = [0u32; FB_WIDTH*FB_HEIGHT];
    let mut micro_seconds_per_frame: i64 = 1000000 / 60;
    system.poweron();
    while window.is_open() {
        let start = PreciseTime::now();
        let mut ascii: u8 = 0;
        let shift = window.is_key_down(Key::LeftShift)|window.is_key_down(Key::RightShift);
        for key in KEYS {
            if window.is_key_down(key.0) {
                ascii = if shift {key.2} else {key.1}
            }
        }
        system.put_key(ascii);
        system.step(micro_seconds_per_frame);
        system.decode_video(&mut frame_buffer);
        window.update_with_buffer(&frame_buffer); 
        let frame_time = start.to(PreciseTime::now());
        micro_seconds_per_frame = frame_time.num_microseconds().unwrap();
    }
}

