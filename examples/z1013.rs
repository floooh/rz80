//
// A minimal Z1013 emulator.
//
// The Z1013 is a very simple Z80-based home computer, just a CPU,
// a PIO, some RAM, ROM and a keyboard matrix: 
//
// Since this is just a minimal sample, some Z1013 features
// are not implemented, most notably cassette tape in/out and 
// sound output. Also, since the Z1013 doesn't require
// interrupts to run, all interrupt handling and
// the interrupt controller daisychain have been left out.
//
// For convenience, a BASIC interpreter has been preloaded (this would
// normally happen by loading from cassette tape). To start the
// BASIC interpreter, type 'J 300[Enter]' on the command prompt.
//
// To write a little BASIC Hello World program (NOTE: currently, an 
// American-English keyboard layout is hardcoded):
//
// >AUTO[Enter]
// 10 FOR I=0 TO 10[Enter]
// 20 PRINT "HELLO WORLD"[Enter]
// 30 NEXT[Enter]
// 40 [Escape]
// >RUN[Enter]
// 
// To leave the BASIC interpreter, type 'BYE[Enter]'

extern crate rz80;
extern crate time;
extern crate minifb;

use rz80::{CPU, PIO, Bus, RegT, PIO_A, PIO_B};
use minifb::{Key, Window, Scale, WindowOptions};
use time::PreciseTime;
use std::cell::RefCell;

// import binary dumps of the operating system, font data and BASIC interpreter
static OS:      &'static [u8] = include_bytes!("z1013_mon_a2.bin");
static FONT:    &'static [u8] = include_bytes!("z1013_font.bin");
static BASIC:   &'static [u8] = include_bytes!("kc_basic.z80"); 

// framebuffer dimensions (32x32 characters @ 8x8 pixels)
const WIDTH: usize=256;
const HEIGHT: usize=256;
// number of entries in key-mapping tables
const MAX_KEYS: usize=128;
// CPU frequency in KHz
const FREQ_KHZ: i64=2000;

// a mapping of all required minifb key codes to their ASCII values, the
// first ASCII value is with shift-key released, the second with shift-key pressed
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

// ASCII codes for the 2 layers of the 8x8 keyboard matrix, the
// first 64 values are with shift-key released, the last 64
// values with shift-key pressed
static KEY_MATRIX: &'static [u8] =
    b"13579-  QETUO@  ADGJL*  YCBM.^  24680[  WRZIP]  SFHK+\\  XVN,/_  \
      !#%')=  qetuo`  adgjl:  ycbm>~  \"$&( {  wrzip}  sfhk;|  xvn<?   ";

// The Z1013 struct holds additional emulator state that's needed
// on top of the pure chip emulation. For the Z1013 emulation,
// all this additional state is related to keyboard input.
//
// Keyboard input on the newer Z1013 models with 8x8 keyboard matrix 
// works like this:
//
// The CPU 'lights up' the keyboard matrix columns by writing
// column number 0..7 to output port 0x08. After each write to output
// port 0x08, the CPU reads back the state of the keyboard matrix
// line by doing 2 separate reads from PIO channel B. 2 separate
// reads are necessary because only 4 bits of the input register
// are reserved for keyboard input (this seems to be a relic from
// the older Z1013 models which only had a 8x4 keyboard matrix).
// To select the 'upper' or 'lower' 4 lines of the keyboard matrix,
// the CPU does a write to PIO-B with bit 4 on or off.
//
// The key_map array member holds the complete 64-bit keyboard matrix 
// state for each possible ASCII code. Whenever a key is in pressed state
// on the host machine, the next_kbd_matrix_bits will be set
// to the corresponding keyboard matrix state. Whenever 
// a new keyboard polling sequence starts in the emulator, this
// next_kbd_matrix_bits mask is copied into the kbd_matrix_bits
// which remains valid until the keyboard polling sequence 
// is finished. The kbd_matrix_bits member is essentially the
// current state of the keyboard matrix, which remains valid
// as long as the keyboard polling function in the Z1013 OS
// reads out the matrix state (this is a bit of a hack, but ensures
// clean keyboard input since the keyboard scanning routine in the
// Z1013 OS will never encounter an inconsistent keyboard matrix state).
//
// The currently scanned keyboard matrix column (that's lit up
// by CPU writes to the output port 0x08) is stored in 
// kbd_column_nr_requested. In addition the bool member
// kbd_high_lines_requested determines whether the upper or
// lower 4 keyboard matrix lines are requested by the CPU
// (by writing to bit 4 of PIO channel B). Together, these two
// members are used to extract the right 4 keyboard matrix line
// bits to return when the CPU reads from PIO channel B.
//
struct Z1013 {
    kbd_column_nr_requested: usize,     // kbd matrix column 'lit up' by CPU
    kbd_high_lines_requested: bool,     // get upper or lower 4 kbd matrix lines
    next_kbd_matrix_bits: u64,          // kbd matrix state of 'next' key
    kbd_matrix_bits: u64,               // kbd matrix state of current key
    key_map: [u64; MAX_KEYS],           // kbd matrix state table for all keys
}

impl Z1013 {
    pub fn new() -> Z1013 {
        Z1013 {
            kbd_column_nr_requested: 0,
            kbd_high_lines_requested: false,
            next_kbd_matrix_bits: 0,
            kbd_matrix_bits: 0,
            key_map: Z1013::key_map(),
        }
    }

    // get the keyboard matrix state bit for a given column and line
    fn key_bit(col: usize, line: usize) -> u64 {
        (1u64<<line)<<(col*8)
    }
    
    // get the matrix state bits for a column/line with shift key status
    fn key_mask(col: usize, line: usize, shift:bool) -> u64 {
        Z1013::key_bit(col, line) | if shift {Z1013::key_bit(7, 6)} else {0}
    }

    // get the entire keyboard matrix state map
    fn key_map() -> [u64; MAX_KEYS] {
        let mut map = [0u64; MAX_KEYS];
        for shift in 0..2 {
            for line in 0..8 {
                for col in 0..8 {
                    let c = KEY_MATRIX[shift*64 + line*8 + col] as usize;
                    if 0x20 != c {
                        map[c] = Z1013::key_mask(col, line, shift != 0);
                    }
                }
            }
        }

        // special keys
        map[0x20] = Z1013::key_bit(6, 4);    // space
        map[0x08] = Z1013::key_bit(6, 2);    // cursor left
        map[0x09] = Z1013::key_bit(6, 3);    // cursor right
        map[0x0A] = Z1013::key_bit(6, 7);    // cursor down
        map[0x0B] = Z1013::key_bit(6, 6);    // cursor up
        map[0x0D] = Z1013::key_bit(6, 1);    // enter

        // Ctrl+C (== STOP/BREAK)
        map[0x03] = Z1013::key_bit(6, 5) | Z1013::key_bit(1, 3);

        map
    }

    // update next keyboard matrix state when a host machine key is pressed
    pub fn put_key(&mut self, ascii: u8) {
        self.next_kbd_matrix_bits = match ascii {
            0 => 0,
            _ => self.key_map[(ascii as usize) & (MAX_KEYS-1)]
        };
    }
}

// The System struct owns all the hardware components and implements the 
// Bus trait, which implements the emulator-specific 'wiring'.
// The use of RefCell here is a bit smelly :/
struct System {
    pub cpu: RefCell<CPU>,
    pub pio: RefCell<PIO>,
    pub z1013: RefCell<Z1013>,
}

// The Bus trait, implemented for the Z1013. This defines how the
// various hardware components in an emulated system talk to each other.
impl Bus for System {

    // cpu_outp() is called when the CPU executes an OUT instruction, on the
    // Z1013 there are 5 important output ports:
    //
    // 0x00:    PIO-A data (unused)
    // 0x01:    PIO-A control (unused)
    // 0x02:    PIO-B data (keyboard input)
    // 0x03:    PIO-B control (keyboard input)
    // 0x08:    light up keyboard matrix columns
    //
    // For the output ports 0x00 to 0x03, the method will simply forward
    // the output value to the respective PIO write function. For
    // port 0x08, the requested keyboard column is stored for later
    // when the CPU reads back the keyboard matrix line state.
    fn cpu_outp(&self, port: RegT, val: RegT) {
        match port & 0xFF {
            0x00 => self.pio.borrow_mut().write_data(self, PIO_A, val),
            0x01 => self.pio.borrow_mut().write_control(PIO_A, val),
            0x02 => self.pio.borrow_mut().write_data(self, PIO_B, val),
            0x03 => self.pio.borrow_mut().write_control(PIO_B, val),
            0x08 => {
                let mut z1013 = self.z1013.borrow_mut();
                if val == 0 {
                    // OS starts reading out a new key
                    z1013.kbd_matrix_bits = z1013.next_kbd_matrix_bits;
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

    // pio_outp() is called when a PIO data register is written,
    // the second '_' parameter is an ID for the PIO, this is
    // only important for emulated systems with multiple PIOs.
    // The only thing that's happening here is checking whether
    // bit 4 is set when writing to PIO-B, this tells us whether
    // the lower or upper 4 keyboard matrix lines are requested
    // in the next read of PIO-B
    fn pio_outp(&self, _: usize, chn: usize, data: RegT) {
        if chn == PIO_B {
            let mut z1013 = self.z1013.borrow_mut();
            z1013.kbd_high_lines_requested = 0 != (data & (1<<4));
        }
    }

    // pio_inp() is called when a PIO data register is read, and this
    // is final piece in the keyboard emulation puzzle, since this
    // is where the upper or lower 4 lines of the keyboard matrix
    // are returned
    fn pio_inp(&self, _: usize, chn: usize) -> RegT {
        if chn == PIO_B {
            let z1013 = self.z1013.borrow();
            let col = z1013.kbd_column_nr_requested & 7;
            let mut val = z1013.kbd_matrix_bits >> (col*8);
            if z1013.kbd_high_lines_requested {
                // upper 4 keyboard matrix lines are requested,
                // shift the bit down into place
                val >>= 4;
            }
            // the keyboard matrix logic is 'active low', so 
            // invert all the relevant bits
            val = 0xF & !(val & 0xF);
            val as RegT
        }
        else {
            // ignore reads from PIO-A
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

    // first-time init of the emulator 
    pub fn poweron(&self) {
        let mut cpu = self.cpu.borrow_mut();
        
        // map 64 KByte RAM at memory layer 1
        cpu.mem.map(1, 0x00000, 0x0000, true, 0x10000);

        // map the 2 KByte OS ROM at higher prio memory layer 0
        cpu.mem.map_bytes(0, 0x10000, 0xF000, false, &OS);

        // copy BASIC interpreter dump into RAM at address 0x100, 
        // skip the first 0x20 bytes, these are used as header
        // of the '.z80' file format
        cpu.mem.write(0x0100, &BASIC[0x20..]);

        // start execution at address 0xF000
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

    // Decode the 32x32 video memory (at address 0xEC00 to 0xEFFF) into a 
    // linear RGBA8 frame buffer, each byte stores an 'extended ASCII code'. 
    // The 'system font' pixel data lives in a hidden ROM not accessible 
    // by the CPU.
    pub fn decode_framebuffer(&self, fb: &mut [u32]) {
        let mut fb_iter = fb.iter_mut();
        let cpu = self.cpu.borrow();
        let vid_mem = &cpu.mem.heap[0xEC00..0xF000];
        for y in 0..32 {
            for py in 0..8 {
                for x in 0..32 {
                    let chr = vid_mem[(y<<5)+x] as usize;
                    let bits = FONT[(chr<<3)|py];
                    for px in 0..8 {
                        let pixel = if (bits & (0x80>>px)) != 0 {
                            0xFFFFFFFF
                        } 
                        else {
                            0xFF000000
                        };
                        *fb_iter.next().unwrap() = pixel;
                    }
                }
            }
        }
    }

    // forward a new host ASCII key code to the emulator
    pub fn put_key(&mut self, ascii: u8) {
        let mut z1013 = self.z1013.borrow_mut();
        z1013.put_key(ascii);
    }
}

//--- the main loop
fn main() {
    // create a window via minifb
    let mut window = match Window::new("rz80 Z1013 Example",
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
    
    // spin up the emulator and run the main loop
    let mut system = System::new();
    system.poweron();
    let mut micro_seconds_per_frame: i64 = 0;
    while window.is_open() {
        let start = PreciseTime::now();

        // get keyboard input from minifb, this is currently a bit crude...
        let mut ascii: u8 = 0;
        let shift = window.is_key_down(Key::LeftShift)|window.is_key_down(Key::RightShift);
        for key in KEYS {
            if window.is_key_down(key.0) {
                ascii = if shift {key.2} else {key.1}
            }
        }
        system.put_key(ascii);

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

