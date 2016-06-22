use memory::Memory;

/// Z80 status flags
const CF : i32 = 1<<0;      // carry flag
const NF : i32 = 1<<1;      // add/subtract flag
const VF : i32 = 1<<2;      // parity/overflow flag
//const PF : i32 = 1<<2;      // parity/overflow flag
const XF : i32 = 1<<3;      // undocumented flag bit 3
const HF : i32 = 1<<4;      // half-carry flag
const YF : i32 = 1<<5;      // undocumented flag bit 5
const ZF : i32 = 1<<6;      // zero flag
const SF : i32 = 1<<7;      // sign flag

/// the Z80 CPU state
pub struct CPU {
    pub a: u8, pub f: u8,
    pub b: u8, pub c: u8,
    pub d: u8, pub e: u8,
    pub h: u8, pub l: u8,
    
    pub ix: u16,
    pub iy: u16,
    pub wz: u16,

    pub af_: u16,
    pub bc_: u16,
    pub de_: u16,
    pub hl_: u16,
    pub wz_: u16,

    pub sp: u16,
    pub pc: u16,
    
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
            a: 0, f: 0, b: 0, c: 0, d: 0, e: 0, h: 0, l: 0,
            ix: 0, iy: 0, wz: 0, 
            af_: 0, bc_: 0, de_: 0, hl_: 0, wz_: 0,
            sp: 0, pc: 0,
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

    /// go into HALT state
    pub fn halt(&mut self) {
        self.halt = true;
        self.pc -= 1;
    }

    /// implement RST instruction
    pub fn rst(&mut self, v: u8) {
        self.sp -= 2;
        self.mem.w16(self.sp, self.pc);
        self.pc = v as u16;
        self.wz = self.pc;
    }

    /// compute flags for add/adc
    pub fn flags_add(acc: i32, add: i32, res: i32) -> u8 {
        let f : i32 = 
            (if (res & 0xFF)==0 { SF } else { res & ZF }) |
            (res & (YF|XF)) |
            ((res>>8) & CF) |            
            ((acc^add^res) & HF) |
            ((((acc^add^0x80) & (add^res))>>5) & VF);
        f as u8
    }

    /// compute flags for sub/sbc
    pub fn flags_sub(acc: i32, sub: i32, res: i32) -> u8 {
        let f : i32 = NF | 
            (if (res & 0xFF)==0 { SF } else { res & ZF }) |
            (res & (YF|XF)) |
            ((res>>8) & CF) |            
            ((acc^sub^res) & HF) |
            ((((acc^sub)&(res^acc))>>5)&VF);
        f as u8
    }

}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn cpu_new() {
        let cpu = CPU::new();
        assert!((0 == cpu.a) && (0 == cpu.f));
        assert!((0 == cpu.b) && (0 == cpu.c));
        assert!((0 == cpu.d) && (0 == cpu.e));
        assert!((0 == cpu.h) && (0 == cpu.l));
        assert!((0 == cpu.ix) && (0 == cpu.iy));
        assert!(0 == cpu.wz);
        assert!((0 == cpu.af_) && (0 == cpu.bc_));
        assert!((0 == cpu.de_) && (0 == cpu.hl_));
        assert!(0 == cpu.wz_);
        assert!((0 == cpu.sp) && (0 == cpu.pc));
        assert!((0 == cpu.i) && (0 == cpu.r));
        assert!(0 == cpu.im);
        assert!(!cpu.halt);
        assert!(!cpu.iff1);
        assert!(!cpu.iff2);
    }

    #[test]
    fn cpu_reset() {
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
    fn cpu_halt() {
        let mut cpu = CPU::new();
        cpu.pc = 0x1234;
        cpu.halt();
        assert!(cpu.halt);
        assert!(0x1233 == cpu.pc);
    }

    #[test]
    fn cpu_rst() {
        let mut cpu = CPU::new();
        cpu.pc = 0x123;
        cpu.sp = 0x100;
        cpu.rst(0x38);
        assert!(0xFE == cpu.sp);
        assert!(cpu.mem.r16(cpu.sp) == 0x123);
        assert!(0x38 == cpu.pc);
        assert!(0x38 == cpu.wz);
    }
}

