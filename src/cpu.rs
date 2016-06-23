use memory::Memory;

/// Z80 status flags
pub const CF : i32 = 1<<0;      // carry flag
pub const NF : i32 = 1<<1;      // add/subtract flag
pub const VF : i32 = 1<<2;      // parity/overflow flag
//const PF : i32 = 1<<2;      // parity/overflow flag
pub const XF : i32 = 1<<3;      // undocumented flag bit 3
pub const HF : i32 = 1<<4;      // half-carry flag
pub const YF : i32 = 1<<5;      // undocumented flag bit 5
pub const ZF : i32 = 1<<6;      // zero flag
pub const SF : i32 = 1<<7;      // sign flag

/// 8-bit register indices
pub const B : usize = 0;
pub const C : usize = 1;
pub const D : usize = 2;
pub const E : usize = 3;
pub const H : usize = 4;
pub const L : usize = 5;
pub const F : usize = 6;
pub const A : usize = 7;
pub const IXH : usize = 8;
pub const IXL : usize = 9;
pub const IYH : usize = 10;
pub const IYL : usize = 11;
pub const SPH : usize = 12;
pub const SPL : usize = 13;
pub const NUM_REGS : usize = 14;

/// 16-bit register indices
pub const BC : usize = 0;
pub const DE : usize = 2;
pub const HL : usize = 4;
pub const AF : usize = 6;
pub const IX : usize = 8;
pub const IY : usize = 10;
pub const SP : usize = 12;

/// the Z80 CPU state
pub struct CPU {
    pub reg : [u8; NUM_REGS],

    pub wz: u16,
    pub pc: u16,
    
    pub af_: u16,
    pub bc_: u16,
    pub de_: u16,
    pub hl_: u16,
    pub wz_: u16,
    
    pub i: u8,
    pub r: u8,
    pub im: u8,

    pub halt: bool,
    pub iff1: bool,
    pub iff2: bool,

    pub mem : Memory,
}

impl CPU {
    /// initialize a new Z80 CPU object
    pub fn new() -> CPU {
        CPU {
            reg: [0; NUM_REGS],
            af_: 0, bc_: 0, de_: 0, hl_: 0, wz_: 0,
            wz: 0, pc: 0,
            i: 0, r: 0, im: 0,
            halt: false, iff1: false, iff2: false,
            mem: Memory::new()
        }
    }

    /// reset the cpu
    pub fn reset(&mut self) {
        self.pc = 0;
        self.wz = 0;
        self.im = 0;
        self.halt = false;
        self.iff1 = false;
        self.iff2 = false;
        self.i = 0;
        self.r = 0;
    }

    /// write 16-bit register value
    pub fn w16(&mut self, r: usize, val: u16) {
        self.reg[r]   = (val>>8) as u8;
        self.reg[r+1] = (val&0xFF) as u8; 
    }

    /// read 16-bit register value
    pub fn r16(&self, r: usize) -> u16 {
        let h : u16 = (self.reg[r] as u16) <<8;
        let l : u16 = self.reg[r+1] as u16;
        h | l
    }

    /// go into HALT state
    pub fn halt(&mut self) {
        self.halt = true;
        self.pc -= 1;
    }

    /// push a value on the stack
    pub fn push(&mut self, val: u16) {
        let sp = self.r16(SP).wrapping_sub(2);
        self.w16(SP, sp);
        self.mem.w16(sp, val);
    }

    /// implement RST instruction
    pub fn rst(&mut self, val: u8) {
        // workaround for https://github.com/rust-lang/rust/issues/29975
        let pc = self.pc;
        self.push(pc);
        self.pc = val as u16;
        self.wz = self.pc;
    }

    /// compute flags for add/adc
    pub fn flags_add(acc: i32, add: i32, res: i32) -> u8 {
        let f : i32 = 
            (if (res & 0xFF)==0 { ZF } else { res & SF }) |
            (res & (YF|XF)) |
            ((res>>8) & CF) |            
            ((acc^add^res) & HF) |
            ((((acc^add^0x80) & (add^res))>>5) & VF);
        f as u8
    }

    /// compute flags for sub/sbc
    pub fn flags_sub(acc: i32, sub: i32, res: i32) -> u8 {
        let f : i32 = NF | 
            (if (res & 0xFF)==0 { ZF } else { res & SF }) |
            (res & (YF|XF)) |
            ((res>>8) & CF) |            
            ((acc^sub^res) & HF) |
            ((((acc^sub)&(res^acc))>>5)&VF);
        f as u8
    }

    /// 8-bit addition
    pub fn add8(&mut self, v: u8) {
        let acc = self.reg[A] as i32;
        let add = v as i32;
        let res = acc + add;
        self.reg[F] = CPU::flags_add(acc, add, res);
        self.reg[A] = (res&0xFF) as u8;
    }

    /// 8-bit addition with carry
    pub fn adc8(&mut self, v: u8) {
        let acc = self.reg[A] as i32;
        let add = v as i32;
        let res = acc + add + ((self.reg[F] as i32) & CF);
        self.reg[F] = CPU::flags_add(acc, add, res);
        self.reg[A] = (res&0xFF) as u8;
    }

    /// 8-bit subtraction
    pub fn sub8(&mut self, v: u8) {
        let acc = self.reg[A] as i32;
        let sub = v as i32;
        let res = acc - sub;
        self.reg[F] = CPU::flags_sub(acc, sub, res);
        self.reg[A] = (res&0xFF) as u8;
    }

    /// 8-bit subtraction with carry
    pub fn sbc8(&mut self, v: u8) {
        let acc = self.reg[A] as i32;
        let sub = v as i32;
        let res = acc - sub - ((self.reg[F] as i32) & CF);
        self.reg[F] = CPU::flags_sub(acc, sub, res);
        self.reg[A] = (res&0xFF) as u8;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn new() {
        let cpu = CPU::new();
        assert!((0 == cpu.reg[A]) && (0 == cpu.reg[F]));
        assert!((0 == cpu.reg[B]) && (0 == cpu.reg[C]));
        assert!((0 == cpu.reg[D]) && (0 == cpu.reg[E]));
        assert!((0 == cpu.reg[H]) && (0 == cpu.reg[L]));
        assert!((0 == cpu.reg[IXH]) && (0 == cpu.reg[IXL]));
        assert!((0 == cpu.reg[IYH]) && (0 == cpu.reg[IYL]));
        assert!((0 == cpu.reg[SPH]) && (0 == cpu.reg[SPL]));
        assert!(0 == cpu.wz);
        assert!(0 == cpu.pc);
        assert!((0 == cpu.af_) && (0 == cpu.bc_));
        assert!((0 == cpu.de_) && (0 == cpu.hl_));
        assert!(0 == cpu.wz_);
        assert!((0 == cpu.i) && (0 == cpu.r));
        assert!(0 == cpu.im);
        assert!(!cpu.halt);
        assert!(!cpu.iff1);
        assert!(!cpu.iff2);
    }

    #[test]
    fn reset() {
        let mut cpu = CPU::new();
        cpu.pc = 0x1234;
        cpu.wz = 1234;
        cpu.im = 45;
        cpu.halt = true;
        cpu.iff1 = true;
        cpu.iff2 = true;
        cpu.i = 2;
        cpu.r = 3;
        cpu.reset();
        assert!(0 == cpu.pc);
        assert!(0 == cpu.wz);
        assert!(0 == cpu.im);
        assert!(!cpu.halt);
        assert!(!cpu.iff1);
        assert!(!cpu.iff2);
        assert!(0 == cpu.i);
        assert!(0 == cpu.r);
    }

    #[test]
    fn reg16_rw() {
        let mut cpu = CPU::new();
        cpu.w16(BC, 0x1234);
        cpu.w16(DE, 0x5678);
        cpu.w16(HL, 0x1357);
        cpu.w16(AF, 0x1122);
        assert!(0x12 == cpu.reg[B]);
        assert!(0x34 == cpu.reg[C]);
        assert!(0x1234 == cpu.r16(BC));
        assert!(0x56 == cpu.reg[D]);
        assert!(0x78 == cpu.reg[E]);
        assert!(0x5678 == cpu.r16(DE));
        assert!(0x13 == cpu.reg[H]);
        assert!(0x57 == cpu.reg[L]);
        assert!(0x1357 == cpu.r16(HL));
        assert!(0x11 == cpu.reg[F]);
        assert!(0x22 == cpu.reg[A]);
    }

    #[test]
    fn halt() {
        let mut cpu = CPU::new();
        cpu.pc = 0x1234;
        cpu.halt();
        assert!(cpu.halt);
        assert!(0x1233 == cpu.pc);
    }

    #[test]
    fn rst() {
        let mut cpu = CPU::new();
        cpu.pc = 0x123;
        cpu.w16(SP, 0x100);
        cpu.rst(0x38);
        assert!(0xFE == cpu.r16(SP));
        assert!(cpu.mem.r16(cpu.r16(SP)) == 0x123);
        assert!(0x38 == cpu.pc);
        assert!(0x38 == cpu.wz);
    }

    #[test]
    fn push() {
        let mut cpu = CPU::new();
        cpu.w16(SP, 0x100);
        cpu.push(0x1234);
        assert!(0xFE == cpu.r16(SP));
        assert!(cpu.mem.r16(cpu.r16(SP)) == 0x1234);
    }

    fn test_flags(flags: u8, mask: i32) -> bool {
        (((flags as i32) & !(XF|YF)) & mask) == mask
    }

    #[test]
    fn add8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0xF;
        cpu.add8(0xF);
        assert!(0x1E == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], HF));
        cpu.add8(0xE0);
        assert!(0xFE == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], SF));
        cpu.reg[A] = 0x81;
        cpu.add8(0x80);
        assert!(0x01 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], VF|CF));
        cpu.add8(0xFF);
        assert!(0x00 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], ZF|HF|CF));
    }

    #[test]
    fn adc8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0x00;
        cpu.adc8(0x00);
        assert!(0x00 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], ZF));
        cpu.adc8(0x41);
        assert!(0x41 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], 0));
        cpu.adc8(0x61);
        assert!(0xA2 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], SF|VF));
        cpu.adc8(0x81);
        assert!(0x23 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], VF|CF));
        cpu.adc8(0x41);
        assert!(0x65 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], 0));
    }

    #[test]
    fn sub8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0x04;
        cpu.sub8(0x04);
        assert!(0x00 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], ZF|NF));
        cpu.sub8(0x01);
        assert!(0xFF == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], SF|HF|NF|CF));
        cpu.sub8(0xF8);
        assert!(0x07 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], NF));
        cpu.sub8(0x0F);
        assert!(0xF8 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], SF|HF|NF|CF));
    }

    #[test]
    fn sbc8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0x04;
        cpu.sbc8(0x04);
        assert!(0x00 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], ZF|NF));
        cpu.sbc8(0x01);
        assert!(0xFF == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], SF|HF|NF|CF));
        cpu.sbc8(0xF8);
        assert!(0x06 == cpu.reg[A]);
        assert!(test_flags(cpu.reg[F], NF));
    }
}

