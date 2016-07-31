//! **rz80** is a Z80 chip family emulation library written in Rust which can be used as basis for
//! writing a full Z80-based computer emulator.
//!
//! # Overview
//!
//! The rz80 library provides chip emulators for the Z80 **CPU**, **PIO** (parallel in/out), **CTC**
//! (counter/timer channels) and a **Bus** trait which defines how the chips are wired together
//! in a specific emulated system.
//!
//! Writing a home computer emulator usually involves the following steps
//!
//! - import the required chips
//! - import ROM dumps using the include_bytes! macro
//! - define a **State** struct which holds emulator state required in addition to the chip state
//! - define a **System** struct which embeds all chips and the State struct wrapped in RefCells
//! - write a **System::poweron()** function which initializes the embedded chips and state objects,
//!   initializes the memory map and sets the CPU PC register to the ROM dump start address
//! - write a **video-decoder** function which generates a linear RGBA8 framebuffer each frame
//! - implement the **Bus trait** on the System struct, this usually involves:
//!     - the keyboard emulation
//!     - memory bank switching
//!     - forward interrupt requests between the various hardware components
//!     - sound generation
//! - implement the **main loop** which creates a window, forwards keyboard input,
//!   and steps the chips emulators forward
//!
//! Very simple 8-bit home computer systems (similar to the ZX81) don't require any additional
//! code, more complex home computers will require additional custom chips emulations that
//! are not part of the rz80 library.
//!
//! Check out the two included example emulators:
//!
//! ```bash 
//! > cargo run --release --example z1013
//! > cargo run --release --example kc87
//! ```
//!

/// generic integer type for 8- and 16-bit values
pub type RegT = i32;

mod registers;
mod memory;
mod bus;
mod cpu;
mod pio;
mod ctc;
mod daisychain;

pub use registers::{Registers, CF, NF, VF, PF, XF, HF, YF, ZF, SF};
pub use memory::Memory;
pub use cpu::CPU;
pub use bus::Bus;
pub use pio::{PIO, PIO_A, PIO_B};
pub use ctc::{CTC, CTC_0, CTC_1, CTC_2, CTC_3};
pub use daisychain::Daisychain;
