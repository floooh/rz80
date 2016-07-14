/// type aliases for 8- and 16-bit registers
/// to prevent excessive type casting
pub type RegT = i32;

pub mod registers;
pub mod memory;
pub mod cpu;

pub use registers::Registers as Registers;
pub use memory::Memory as Memory;
pub use cpu::CPU as CPU;

pub use registers::CF as CF;
pub use registers::NF as NF;
pub use registers::VF as VF;
pub use registers::PF as PF;
pub use registers::XF as XF;
pub use registers::HF as HF;
pub use registers::YF as YF;
pub use registers::ZF as ZF;
pub use registers::SF as SF;

