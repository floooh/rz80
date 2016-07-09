#![allow(dead_code)]

use RegT;

/// Z80 status flags
pub const CF : RegT = 1<<0;      // carry flag
pub const NF : RegT = 1<<1;      // add/subtract flag
pub const VF : RegT = 1<<2;      // parity/overflow flag
pub const PF : RegT = 1<<2;      // parity/overflow flag
pub const XF : RegT = 1<<3;      // undocumented flag bit 3
pub const HF : RegT = 1<<4;      // half-carry flag
pub const YF : RegT = 1<<5;      // undocumented flag bit 5
pub const ZF : RegT = 1<<6;      // zero flag
pub const SF : RegT = 1<<7;      // sign flag

const B : usize = 0;
const C : usize = 1;
const D : usize = 2;
const E : usize = 3;
const H : usize = 4;
const L : usize = 5;
const A : usize = 6;
const F : usize = 7;
const IXH : usize = 8;
const IXL : usize = 9;
const IYH : usize = 10;
const IYL : usize = 11;
const SPH : usize = 12;
const SPL : usize = 13;
const WZH : usize = 14;
const WZL : usize = 15;
const B_ : usize = 16;
const C_ : usize = 17;
const D_ : usize = 18;
const E_ : usize = 19;
const H_ : usize = 20;
const L_ : usize = 21;
const F_ : usize = 22;
const A_ : usize = 23;
const WZH_ : usize = 24;
const WZL_ : usize = 25;
const NUM_REGS : usize = 26;

pub const BC : usize = 0;
pub const DE : usize = 2;
pub const HL : usize = 4;
pub const AF : usize = 6;
const IX : usize = 8;
const IY : usize = 10;
const SP : usize = 12;
pub const WZ : usize = 14;
pub const BC_ : usize = 16;
pub const DE_ : usize = 18;
pub const HL_ : usize = 20;
pub const AF_ : usize = 22;
pub const WZ_ : usize = 24;

/// Z80 register bank
pub struct Registers {
    reg : [RegT; NUM_REGS],
    r_pc: RegT,
    
    pub i: RegT,
    pub r: RegT,
    pub im: RegT,

    m_r  : [usize; 8],
    m_r2 : [usize; 8],
    m_sp : [usize; 4],
    m_af : [usize; 4],
}

impl Registers {
    /// initialize a new Registers object
    pub fn new() -> Registers {
        Registers {
            reg: [0; NUM_REGS],
            r_pc: 0,
            i: 0, r: 0, im: 0,
            m_r  : [ B, C, D, E, H, L, F, A ],
            m_r2 : [ B, C, D, E, H, L, F, A ],
            m_sp : [ BC, DE, HL, SP ],
            m_af : [ BC, DE, HL, AF ],
        }
    }

    /// perform a CPU reset (clears some, but not all registers)
    pub fn reset(&mut self) {
        self.r_pc = 0;
        self.set_wz(0);
        self.im = 0;
        self.i = 0;
        self.r = 0;
    }

    /// get 8-bit registers
    pub fn a(&self) -> RegT { self.reg[A] }
    pub fn f(&self) -> RegT { self.reg[F] }
    pub fn b(&self) -> RegT { self.reg[B] }
    pub fn c(&self) -> RegT { self.reg[C] }
    pub fn d(&self) -> RegT { self.reg[D] }
    pub fn e(&self) -> RegT { self.reg[E] }
    pub fn h(&self) -> RegT { self.reg[H] }
    pub fn l(&self) -> RegT { self.reg[L] }
    pub fn w(&self) -> RegT { self.reg[WZH] }

    /// set 8-bit registers
    pub fn set_a(&mut self, v: RegT) { self.reg[A] = v & 0xFF; }
    pub fn set_f(&mut self, v: RegT) { self.reg[F] = v & 0xFF; }
    pub fn set_b(&mut self, v: RegT) { self.reg[B] = v & 0xFF; }
    pub fn set_c(&mut self, v: RegT) { self.reg[C] = v & 0xFF; }
    pub fn set_d(&mut self, v: RegT) { self.reg[D] = v & 0xFF; }
    pub fn set_e(&mut self, v: RegT) { self.reg[E] = v & 0xFF; }
    pub fn set_h(&mut self, v: RegT) { self.reg[H] = v & 0xFF; }
    pub fn set_l(&mut self, v: RegT) { self.reg[L] = v & 0xFF; }

    /// get 16-bit registers
    pub fn af(&self) -> RegT { self.reg[A]<<8 | self.reg[F] }
    pub fn bc(&self) -> RegT { self.reg[B]<<8 | self.reg[C] }
    pub fn de(&self) -> RegT { self.reg[D]<<8 | self.reg[E] }
    pub fn hl(&self) -> RegT { self.reg[H]<<8 | self.reg[L] }
    pub fn ix(&self) -> RegT { self.reg[IXH]<<8 | self.reg[IXL] }
    pub fn iy(&self) -> RegT { self.reg[IYH]<<8 | self.reg[IYL] }
    pub fn sp(&self) -> RegT { self.reg[SPH]<<8 | self.reg[SPL] }
    pub fn wz(&self) -> RegT { self.reg[WZH]<<8 | self.reg[WZL] }
    pub fn af_(&self) -> RegT { self.reg[A_]<<8 | self.reg[F_] }
    pub fn bc_(&self) -> RegT { self.reg[B_]<<8 | self.reg[C_] }
    pub fn de_(&self) -> RegT { self.reg[D_]<<8 | self.reg[E_] }
    pub fn hl_(&self) -> RegT { self.reg[H_]<<8 | self.reg[L_] }
    pub fn wz_(&self) -> RegT { self.reg[WZH_]<<8 | self.reg[WZL_] }
    pub fn pc(&self) -> RegT { self.r_pc }

    /// set 16-bit registers
    pub fn set_af(&mut self, v: RegT) { self.reg[A] = (v>>8) & 0xFF; self.reg[F] = v & 0xFF; }
    pub fn set_bc(&mut self, v: RegT) { self.reg[B] = (v>>8) & 0xFF; self.reg[C] = v & 0xFF; }
    pub fn set_de(&mut self, v: RegT) { self.reg[D] = (v>>8) & 0xFF; self.reg[E] = v & 0xFF; }
    pub fn set_hl(&mut self, v: RegT) { self.reg[H] = (v>>8) & 0xFF; self.reg[L] = v & 0xFF; }
    pub fn set_ix(&mut self, v: RegT) { self.reg[IXH] = (v>>8) & 0xFF; self.reg[IXL] = v & 0xFF; }
    pub fn set_iy(&mut self, v: RegT) { self.reg[IYH] = (v>>8) & 0xFF; self.reg[IYL] = v & 0xFF; }
    pub fn set_sp(&mut self, v: RegT) { self.reg[SPH] = (v>>8) & 0xFF; self.reg[SPL] = v & 0xFF; }
    pub fn set_wz(&mut self, v: RegT) { self.reg[WZH] = (v>>8) & 0xFF; self.reg[WZL] = v & 0xFF; }
    pub fn set_af_(&mut self, v: RegT) { self.reg[A_] = (v>>8) & 0xFF; self.reg[F_] = v & 0xFF; }
    pub fn set_bc_(&mut self, v: RegT) { self.reg[B_] = (v>>8) & 0xFF; self.reg[C_] = v & 0xFF; }
    pub fn set_de_(&mut self, v: RegT) { self.reg[D_] = (v>>8) & 0xFF; self.reg[E_] = v & 0xFF; }
    pub fn set_hl_(&mut self, v: RegT) { self.reg[H_] = (v>>8) & 0xFF; self.reg[L_] = v & 0xFF; }
    pub fn set_wz_(&mut self, v: RegT) { self.reg[WZH_] = (v>>8) & 0xFF; self.reg[WZL_] = v & 0xFF; }
    pub fn set_pc(&mut self, v: RegT) { self.r_pc = v & 0xFFFF; }

    /// get 8-bit register by index (where index is 3-bit register id from Z80 instruction)
    pub fn r8(&self, r: usize) -> RegT { 
        self.reg[self.m_r[r]] 
    }

    /// set 8-bit register by index (where index is 3-bit register id from Z80 instruction)
    pub fn set_r8(&mut self, r: usize, v: RegT) {
        self.reg[self.m_r[r]] = v & 0xFF;
    }

    /// get 8-bit register by index, H,L never patched to IXH,IXL,IYH,IYL
    pub fn r8i(&self, r: usize) -> RegT {
        self.reg[self.m_r2[r]]
    }

    /// set 8-bit register by index, H,L never patched to IXH,IXL,IYH,IYL
    pub fn set_r8i(&mut self, r: usize, v: RegT) {
        self.reg[self.m_r2[r]] = v & 0xFF;
    }

    /// get 16-bit register by direct index (AF, BC, DE, HL, etc)
    pub fn r16i(&self, i: usize) -> RegT {
        self.reg[i]<<8 | self.reg[i+1]
    }

    /// set 16-bit register by direct index (AF, BC, DE, ...)
    pub fn set_r16i(&mut self, i: usize, v: RegT) {
        self.reg[i]   = (v>>8) & 0xFF;
        self.reg[i+1] = v & 0xFF;
    }

    /// get 16-bit register by 2-bit index with mapping through SP-table
    pub fn r16sp(&self, r: usize) -> RegT {
        let i = self.m_sp[r];
        self.r16i(i)
    }
    
    /// set 16-bit register by 2-bit index with mapping through SP-table
    pub fn set_r16sp(&mut self, r: usize, v: RegT) {
        let i = self.m_sp[r];
        self.set_r16i(i, v);
    }

    /// get 16-bit register by 2-bit index with mapping through AF-table
    pub fn r16af(&self, r: usize) -> RegT {
        let i = self.m_af[r];
        self.r16i(i)
    }

    /// set 16-bit register by 2-bit index with mapping through AF-table
    pub fn set_r16af(&mut self, r: usize, v: RegT) {
        let i = self.m_af[r];
        self.set_r16i(i, v);
    }

    /// swap 2 16-bit registers by direct index (HL, BC, DE, ...)
    pub fn swap(&mut self, i: usize, i_: usize) {
        let v  = self.r16i(i);
        let v_ = self.r16i(i_);
        self.set_r16i(i, v_);
        self.set_r16i(i_, v);
    }

    /// patch register mapping tables for use of IX instead of HL
    pub fn patch_ix(&mut self) {
        self.m_r[H]  = IXH;
        self.m_r[L]  = IXL;
        self.m_sp[2] = IX;
        self.m_af[2] = IX;
    }

    /// patch register mapping tables for use of IY instead of HL
    pub fn patch_iy(&mut self) {
        self.m_r[H]  = IYH;
        self.m_r[L]  = IYL;
        self.m_sp[2] = IY;
        self.m_af[2] = IY;
    }

    /// unpatch register mapping tables to use HL instead of IX/IY
    pub fn unpatch(&mut self) {
        self.m_r[H]  = H;
        self.m_r[L]  = L;
        self.m_sp[2] = HL;
        self.m_af[2] = HL;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn new() {
        let reg = Registers::new();
        assert!(reg.a() == 0); assert!(reg.f() == 0);
        assert!(reg.b() == 0); assert!(reg.c() == 0);
        assert!(reg.d() == 0); assert!(reg.e() == 0);
        assert!(reg.h() == 0); assert!(reg.l() == 0);
        assert!(reg.af() == 0); assert!(reg.af_() == 0);
        assert!(reg.bc() == 0); assert!(reg.bc_() == 0);
        assert!(reg.de() == 0); assert!(reg.de_() == 0);
        assert!(reg.hl() == 0); assert!(reg.hl_() == 0);
        assert!(reg.wz() == 0); assert!(reg.wz_() == 0);
        assert!(reg.ix() == 0); assert!(reg.iy() == 0);
        assert!(reg.pc() == 0); assert!(reg.sp() == 0);
        assert!(reg.r == 0);
        assert!(reg.i == 0);
        assert!(reg.im == 0);
    }

    #[test]
    fn set_get() {
        let mut reg = Registers::new();
        reg.set_a(0x12); reg.set_f(0x34); 
        assert!(reg.a() == 0x12); assert!(reg.f() == 0x34); assert!(reg.af() == 0x1234);
        reg.set_af(0x2345); 
        assert!(reg.af() == 0x2345); assert!(reg.a() == 0x23); assert!(reg.f() == 0x45);
        reg.set_b(0x34); reg.set_c(0x56);
        assert!(reg.b() == 0x34); assert!(reg.c() == 0x56); assert!(reg.bc() == 0x3456);
        reg.set_d(0x78); reg.set_e(0x9A);
        assert!(reg.de() == 0x789A); assert!(reg.d() == 0x78); assert!(reg.e() == 0x9A);
        reg.set_h(0xAB); reg.set_l(0xCD);
        assert!(reg.hl() == 0xABCD); assert!(reg.h() == 0xAB); assert!(reg.l() == 0xCD);
        reg.set_ix(0x0102);
        assert!(reg.ix() == 0x0102); 
        reg.set_iy(0x0304);
        assert!(reg.iy() == 0x0304);
        reg.set_pc(0x1122);
        assert!(reg.pc() == 0x1122);
        reg.set_sp(0x3344);
        assert!(reg.sp() == 0x3344);
    }
}

