/// type aliases for 8- and 16-bit registers
/// to prevent excessive type casting
pub type RegT = i64;

pub mod memory;
pub mod cpu;

pub use memory::Memory as Memory;
pub use cpu::CPU as CPU;

pub use cpu::B as B;
pub use cpu::C as C;
pub use cpu::D as D;
pub use cpu::E as E;
pub use cpu::H as H;
pub use cpu::L as L;
pub use cpu::F as F;
pub use cpu::A as A;

pub use cpu::BC as BC;
pub use cpu::DE as DE;
pub use cpu::HL as HL;
pub use cpu::AF as AF;
pub use cpu::IX as IX;
pub use cpu::IY as IY;
pub use cpu::SP as SP;
