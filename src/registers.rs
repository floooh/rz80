use RegT;

/// CPU carry flag
pub const CF: RegT = 1 << 0;
/// CPU add/subtract flag
pub const NF: RegT = 1 << 1;
/// CPU overflow flag (same as parity)
pub const VF: RegT = 1 << 2;
/// CPU parity flag (same as overflow)
pub const PF: RegT = 1 << 2;
/// CPU undocumented 'X' flag
pub const XF: RegT = 1 << 3;
/// CPU half carry flag
pub const HF: RegT = 1 << 4;
/// CPU undocumented 'Y' flag
pub const YF: RegT = 1 << 5;
/// CPU zero flag
pub const ZF: RegT = 1 << 6;
/// CPU sign flag
pub const SF: RegT = 1 << 7;

const B: usize = 0;
const C: usize = 1;
const D: usize = 2;
const E: usize = 3;
const H: usize = 4;
const L: usize = 5;
const A: usize = 6;
const F: usize = 7;
const IXH: usize = 8;
const IXL: usize = 9;
const IYH: usize = 10;
const IYL: usize = 11;
const SPH: usize = 12;
const SPL: usize = 13;
const WZH: usize = 14;
const WZL: usize = 15;
const B_: usize = 16;
const C_: usize = 17;
const D_: usize = 18;
const E_: usize = 19;
const H_: usize = 20;
const L_: usize = 21;
const A_: usize = 22;
const F_: usize = 23;
const WZH_: usize = 24;
const WZL_: usize = 25;
const NUM_REGS: usize = 26;

pub const BC: usize = 0;
pub const DE: usize = 2;
pub const HL: usize = 4;
pub const AF: usize = 6;
pub const IX: usize = 8;
pub const IY: usize = 10;
pub const SP: usize = 12;
pub const WZ: usize = 14;
pub const BC_: usize = 16;
pub const DE_: usize = 18;
pub const HL_: usize = 20;
pub const AF_: usize = 22;
pub const WZ_: usize = 24;

/// CPU register access
///
/// # Examples
///
/// set the PC and SP registers:
///
/// ```
/// use rz80::CPU;
///
/// let mut cpu = CPU::new();
/// cpu.reg.set_pc(0x0200);
/// cpu.reg.set_sp(0x01C0);
/// ```
///
/// get the B, C and BC registers
///
/// ```
/// use rz80::CPU;
///
/// let cpu = CPU::new();
/// let b = cpu.reg.b();
/// let c = cpu.reg.c();
/// let bc = cpu.reg.bc();
/// println!("B: {}, C: {}, BC: {}", b, c, bc);
/// ```
/// 8- or 16-bit wraparound happens during the set operation:
///
/// ```
/// use rz80::CPU;
///
/// let mut cpu = CPU::new();
///
/// cpu.reg.set_a(0xFF);
/// let a = cpu.reg.a() + 1;
/// cpu.reg.set_a(a);
/// assert_eq!(cpu.reg.a(), 0x00);
///
/// cpu.reg.set_hl(0x0000);
/// let hl = cpu.reg.hl() - 1;
/// cpu.reg.set_hl(hl);
/// assert_eq!(cpu.reg.hl(), 0xFFFF);
/// ```
pub struct Registers {
    reg: [u8; NUM_REGS],
    r_pc: u16,

    pub i: RegT,
    pub r: RegT,
    pub im: RegT,

    m_r: [usize; 8],
    m_r2: [usize; 8],
    m_sp: [usize; 4],
    m_af: [usize; 4],
}

impl Registers {
    /// initialize a new Registers object
    pub fn new() -> Registers {
        Registers {
            reg: [0; NUM_REGS],
            r_pc: 0,
            i: 0,
            r: 0,
            im: 0,
            m_r: [B, C, D, E, H, L, F, A],
            m_r2: [B, C, D, E, H, L, F, A],
            m_sp: [BC, DE, HL, SP],
            m_af: [BC, DE, HL, AF],
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

    /// get content of A register
    #[inline(always)]
    pub fn a(&self) -> RegT {
        self.reg[A] as RegT
    }
    /// get content of F register (status flags)
    #[inline(always)]
    pub fn f(&self) -> RegT {
        self.reg[F] as RegT
    }
    /// get content of B register
    #[inline(always)]
    pub fn b(&self) -> RegT {
        self.reg[B] as RegT
    }
    /// get content of C register
    #[inline(always)]
    pub fn c(&self) -> RegT {
        self.reg[C] as RegT
    }
    /// get content of D register
    #[inline(always)]
    pub fn d(&self) -> RegT {
        self.reg[D] as RegT
    }
    /// get content of E register
    #[inline(always)]
    pub fn e(&self) -> RegT {
        self.reg[E] as RegT
    }
    /// get content of H register
    #[inline(always)]
    pub fn h(&self) -> RegT {
        self.reg[H] as RegT
    }
    /// get content of L register
    #[inline(always)]
    pub fn l(&self) -> RegT {
        self.reg[L] as RegT
    }
    /// get content of undocumented W register (of WZ register pair)
    #[inline(always)]
    pub fn w(&self) -> RegT {
        self.reg[WZH] as RegT
    }

    /// set content of A register
    #[inline(always)]
    pub fn set_a(&mut self, v: RegT) {
        self.reg[A] = v as u8;
    }
    /// set content of F register (status flags)
    #[inline(always)]
    pub fn set_f(&mut self, v: RegT) {
        self.reg[F] = v as u8;
    }
    /// set content of B register
    #[inline(always)]
    pub fn set_b(&mut self, v: RegT) {
        self.reg[B] = v as u8;
    }
    /// set content of C register
    #[inline(always)]
    pub fn set_c(&mut self, v: RegT) {
        self.reg[C] = v as u8;
    }
    /// set content of D register
    #[inline(always)]
    pub fn set_d(&mut self, v: RegT) {
        self.reg[D] = v as u8;
    }
    /// set content of E register
    #[inline(always)]
    pub fn set_e(&mut self, v: RegT) {
        self.reg[E] = v as u8;
    }
    /// set content of H register
    #[inline(always)]
    pub fn set_h(&mut self, v: RegT) {
        self.reg[H] = v as u8;
    }
    /// set content of L register
    #[inline(always)]
    pub fn set_l(&mut self, v: RegT) {
        self.reg[L] = v as u8;
    }

    /// get content of AF register pair
    #[inline(always)]
    pub fn af(&self) -> RegT {
        (self.reg[A] as RegT) << 8 | self.reg[F] as RegT
    }
    /// get content of BC register pair
    #[inline(always)]
    pub fn bc(&self) -> RegT {
        (self.reg[B] as RegT) << 8 | self.reg[C] as RegT
    }
    /// get content of DE register pair
    #[inline(always)]
    pub fn de(&self) -> RegT {
        (self.reg[D] as RegT) << 8 | self.reg[E] as RegT
    }
    /// get content of HL register pair
    #[inline(always)]
    pub fn hl(&self) -> RegT {
        (self.reg[H] as RegT) << 8 | self.reg[L] as RegT
    }
    /// get content of IX register
    #[inline(always)]
    pub fn ix(&self) -> RegT {
        (self.reg[IXH] as RegT) << 8 | self.reg[IXL] as RegT
    }
    /// get content of IY register
    #[inline(always)]
    pub fn iy(&self) -> RegT {
        (self.reg[IYH] as RegT) << 8 | self.reg[IYL] as RegT
    }
    /// get content of SP register
    #[inline(always)]
    pub fn sp(&self) -> RegT {
        (self.reg[SPH] as RegT) << 8 | self.reg[SPL] as RegT
    }
    /// get content of undocumented WZ register
    #[inline(always)]
    pub fn wz(&self) -> RegT {
        (self.reg[WZH] as RegT) << 8 | self.reg[WZL] as RegT
    }
    /// get content of AF' register
    #[inline(always)]
    pub fn af_(&self) -> RegT {
        (self.reg[A_] as RegT) << 8 | self.reg[F_] as RegT
    }
    /// get content of BC' register
    #[inline(always)]
    pub fn bc_(&self) -> RegT {
        (self.reg[B_] as RegT) << 8 | self.reg[C_] as RegT
    }
    /// get content of DE' register
    #[inline(always)]
    pub fn de_(&self) -> RegT {
        (self.reg[D_] as RegT) << 8 | self.reg[E_] as RegT
    }
    /// get content of HL' register
    #[inline(always)]
    pub fn hl_(&self) -> RegT {
        (self.reg[H_] as RegT) << 8 | self.reg[L_] as RegT
    }
    /// get content of undocumented WZ' register
    #[inline(always)]
    pub fn wz_(&self) -> RegT {
        (self.reg[WZH_] as RegT) << 8 | self.reg[WZL_] as RegT
    }
    /// get content of PC register
    #[inline(always)]
    pub fn pc(&self) -> RegT {
        self.r_pc as RegT
    }

    /// set content of AF register pair
    #[inline(always)]
    pub fn set_af(&mut self, v: RegT) {
        self.reg[A] = (v >> 8) as u8;
        self.reg[F] = v as u8;
    }
    /// set content of BC register pair
    #[inline(always)]
    pub fn set_bc(&mut self, v: RegT) {
        self.reg[B] = (v >> 8) as u8;
        self.reg[C] = v as u8;
    }
    /// set content of DE register pair
    #[inline(always)]
    pub fn set_de(&mut self, v: RegT) {
        self.reg[D] = (v >> 8) as u8;
        self.reg[E] = v as u8;
    }
    /// set content of HL register pair
    #[inline(always)]
    pub fn set_hl(&mut self, v: RegT) {
        self.reg[H] = (v >> 8) as u8;
        self.reg[L] = v as u8;
    }
    /// set content of IX register
    #[inline(always)]
    pub fn set_ix(&mut self, v: RegT) {
        self.reg[IXH] = (v >> 8) as u8;
        self.reg[IXL] = v as u8;
    }
    /// set content of IY register
    #[inline(always)]
    pub fn set_iy(&mut self, v: RegT) {
        self.reg[IYH] = (v >> 8) as u8;
        self.reg[IYL] = v as u8;
    }
    /// set content of SP register
    #[inline(always)]
    pub fn set_sp(&mut self, v: RegT) {
        self.reg[SPH] = (v >> 8) as u8;
        self.reg[SPL] = v as u8;
    }
    /// set content of undocumented WZ register
    #[inline(always)]
    pub fn set_wz(&mut self, v: RegT) {
        self.reg[WZH] = (v >> 8) as u8;
        self.reg[WZL] = v as u8;
    }
    /// set content of AF' register
    #[inline(always)]
    pub fn set_af_(&mut self, v: RegT) {
        self.reg[A_] = (v >> 8) as u8;
        self.reg[F_] = v as u8;
    }
    /// set content of BC' register
    #[inline(always)]
    pub fn set_bc_(&mut self, v: RegT) {
        self.reg[B_] = (v >> 8) as u8;
        self.reg[C_] = v as u8;
    }
    /// set content of DE' register
    #[inline(always)]
    pub fn set_de_(&mut self, v: RegT) {
        self.reg[D_] = (v >> 8) as u8;
        self.reg[E_] = v as u8;
    }
    /// set content of HL' register
    #[inline(always)]
    pub fn set_hl_(&mut self, v: RegT) {
        self.reg[H_] = (v >> 8) as u8;
        self.reg[L_] = v as u8;
    }
    /// set content of undocumented WZ' register
    #[inline(always)]
    pub fn set_wz_(&mut self, v: RegT) {
        self.reg[WZH_] = (v >> 8) as u8;
        self.reg[WZL_] = v as u8;
    }
    /// set content of PC register
    #[inline(always)]
    pub fn set_pc(&mut self, v: RegT) {
        self.r_pc = v as u16;
    }

    /// increment the PC register by some value
    #[inline(always)]
    pub fn inc_pc(&mut self, inc: u16) {
        self.r_pc = self.r_pc.wrapping_add(inc);
    }

    /// decrement the PC register by some value
    #[inline(always)]
    pub fn dec_pc(&mut self, dec: u16) {
        self.r_pc = self.r_pc.wrapping_sub(dec);
    }

    /// get 8-bit register by index (where index is 3-bit register id from Z80 instruction)
    #[inline(always)]
    pub fn r8(&self, r: usize) -> RegT {
        self.reg[self.m_r[r]] as RegT
    }

    /// set 8-bit register by index (where index is 3-bit register id from Z80 instruction)
    #[inline(always)]
    pub fn set_r8(&mut self, r: usize, v: RegT) {
        self.reg[self.m_r[r]] = v as u8;
    }

    /// get 8-bit register by index, H,L never patched to IXH,IXL,IYH,IYL
    #[inline(always)]
    pub fn r8i(&self, r: usize) -> RegT {
        self.reg[self.m_r2[r]] as RegT
    }

    /// set 8-bit register by index, H,L never patched to IXH,IXL,IYH,IYL
    #[inline(always)]
    pub fn set_r8i(&mut self, r: usize, v: RegT) {
        self.reg[self.m_r2[r]] = v as u8;
    }

    /// get 16-bit register by direct index (AF, BC, DE, HL, etc)
    #[inline(always)]
    pub fn r16i(&self, i: usize) -> RegT {
        (self.reg[i] as RegT) << 8 | self.reg[i + 1] as RegT
    }

    /// set 16-bit register by direct index (AF, BC, DE, ...)
    #[inline(always)]
    pub fn set_r16i(&mut self, i: usize, v: RegT) {
        self.reg[i] = (v >> 8) as u8;
        self.reg[i + 1] = v as u8;
    }

    /// get 16-bit register by 2-bit index with mapping through SP-table
    #[inline(always)]
    pub fn r16sp(&self, r: usize) -> RegT {
        let i = self.m_sp[r];
        self.r16i(i)
    }

    /// set 16-bit register by 2-bit index with mapping through SP-table
    #[inline(always)]
    pub fn set_r16sp(&mut self, r: usize, v: RegT) {
        let i = self.m_sp[r];
        self.set_r16i(i, v);
    }

    /// get 16-bit register by 2-bit index with mapping through AF-table
    #[inline(always)]
    pub fn r16af(&self, r: usize) -> RegT {
        let i = self.m_af[r];
        self.r16i(i)
    }

    /// set 16-bit register by 2-bit index with mapping through AF-table
    #[inline(always)]
    pub fn set_r16af(&mut self, r: usize, v: RegT) {
        let i = self.m_af[r];
        self.set_r16i(i, v);
    }

    /// swap 2 16-bit registers by direct index (HL, BC, DE, ...)
    pub fn swap(&mut self, i: usize, i_: usize) {
        let v = self.r16i(i);
        let v_ = self.r16i(i_);
        self.set_r16i(i, v_);
        self.set_r16i(i_, v);
    }

    /// patch register mapping tables for use of IX instead of HL
    pub fn patch_ix(&mut self) {
        self.m_r[H] = IXH;
        self.m_r[L] = IXL;
        self.m_sp[2] = IX;
        self.m_af[2] = IX;
    }

    /// patch register mapping tables for use of IY instead of HL
    pub fn patch_iy(&mut self) {
        self.m_r[H] = IYH;
        self.m_r[L] = IYL;
        self.m_sp[2] = IY;
        self.m_af[2] = IY;
    }

    /// unpatch register mapping tables to use HL instead of IX/IY
    pub fn unpatch(&mut self) {
        self.m_r[H] = H;
        self.m_r[L] = L;
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
        assert_eq!(reg.a(), 0);
        assert_eq!(reg.f(), 0);
        assert_eq!(reg.b(), 0);
        assert_eq!(reg.c(), 0);
        assert_eq!(reg.d(), 0);
        assert_eq!(reg.e(), 0);
        assert_eq!(reg.h(), 0);
        assert_eq!(reg.l(), 0);
        assert_eq!(reg.af(), 0);
        assert_eq!(reg.af_(), 0);
        assert_eq!(reg.bc(), 0);
        assert_eq!(reg.bc_(), 0);
        assert_eq!(reg.de(), 0);
        assert_eq!(reg.de_(), 0);
        assert_eq!(reg.hl(), 0);
        assert_eq!(reg.hl_(), 0);
        assert_eq!(reg.wz(), 0);
        assert_eq!(reg.wz_(), 0);
        assert_eq!(reg.ix(), 0);
        assert_eq!(reg.iy(), 0);
        assert_eq!(reg.pc(), 0);
        assert_eq!(reg.sp(), 0);
        assert_eq!(reg.r, 0);
        assert_eq!(reg.i, 0);
        assert_eq!(reg.im, 0);
    }

    #[test]
    fn set_get() {
        let mut reg = Registers::new();
        reg.set_a(0x12);
        reg.set_f(0x34);
        assert_eq!(reg.a(), 0x12);
        assert_eq!(reg.f(), 0x34);
        assert_eq!(reg.af(), 0x1234);
        reg.set_af(0x2345);
        assert_eq!(reg.af(), 0x2345);
        assert_eq!(reg.a(), 0x23);
        assert_eq!(reg.f(), 0x45);
        reg.set_b(0x34);
        reg.set_c(0x56);
        assert_eq!(reg.b(), 0x34);
        assert_eq!(reg.c(), 0x56);
        assert_eq!(reg.bc(), 0x3456);
        reg.set_d(0x78);
        reg.set_e(0x9A);
        assert_eq!(reg.de(), 0x789A);
        assert_eq!(reg.d(), 0x78);
        assert_eq!(reg.e(), 0x9A);
        reg.set_h(0xAB);
        reg.set_l(0xCD);
        assert_eq!(reg.hl(), 0xABCD);
        assert_eq!(reg.h(), 0xAB);
        assert_eq!(reg.l(), 0xCD);
        reg.set_ix(0x0102);
        assert_eq!(reg.ix(), 0x0102);
        reg.set_iy(0x0304);
        assert_eq!(reg.iy(), 0x0304);
        reg.set_pc(0x1122);
        assert_eq!(reg.pc(), 0x1122);
        reg.set_sp(0x3344);
        assert_eq!(reg.sp(), 0x3344);
    }
}
