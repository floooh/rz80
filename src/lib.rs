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
pub use cpu::BC_ as BC_;
pub use cpu::DE_ as DE_;
pub use cpu::HL_ as HL_;
pub use cpu::AF_ as AF_;

pub use cpu::CF as CF;
pub use cpu::NF as NF;
pub use cpu::VF as VF;
pub use cpu::PF as PF;
pub use cpu::XF as XF;
pub use cpu::HF as HF;
pub use cpu::YF as YF;
pub use cpu::ZF as ZF;
pub use cpu::SF as SF;

