use memory::Memory;
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
pub const B_ : usize = 14;
pub const C_ : usize = 15;
pub const D_ : usize = 16;
pub const E_ : usize = 17;
pub const H_ : usize = 18;
pub const L_ : usize = 19;
pub const F_ : usize = 20;
pub const A_ : usize = 21;
pub const NUM_REGS : usize = 22;

/// 16-bit register indices
pub const BC : usize = 0;
pub const DE : usize = 2;
pub const HL : usize = 4;
pub const AF : usize = 6;
pub const IX : usize = 8;
pub const IY : usize = 10;
pub const SP : usize = 12;
pub const BC_ : usize = 14;
pub const DE_ : usize = 16;
pub const HL_ : usize = 18;
pub const AF_ : usize = 20;

/// the Z80 CPU state
pub struct CPU {
    pub reg : [RegT; NUM_REGS],

    pub wz: RegT,
    pub wz_: RegT,
    pub pc: RegT,
    
    pub i: RegT,
    pub r: RegT,
    pub im: RegT,

    pub halt: bool,
    pub iff1: bool,
    pub iff2: bool,

    pub invalid_op: bool,
    pub enable_interrupt: bool,
    pub mem : Memory,
}

impl CPU {
    /// initialize a new Z80 CPU object
    pub fn new() -> CPU {
        CPU {
            reg: [0; NUM_REGS],
            wz: 0, wz_: 0, pc: 0,
            i: 0, r: 0, im: 0,
            halt: false, iff1: false, iff2: false,
            invalid_op: false,
            enable_interrupt: false,
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
        self.invalid_op = false;
        self.enable_interrupt = false;
    }

    /// write 16-bit register value
    pub fn w16(&mut self, r: usize, val: RegT) {
        self.reg[r]   = (val>>8) & 0xFF;
        self.reg[r+1] = val & 0xFF; 
    }

    /// read 16-bit register value
    pub fn r16(&self, r: usize) -> RegT {
        let h = self.reg[r] <<8;
        let l = self.reg[r+1];
        h | l
    }

    /// fetch the next instruction byte from memory
    fn fetch_op(&mut self) -> RegT {
        self.r = (self.r & 0x80) | ((self.r+1) & 0x7F);
        let op = self.mem.r8(self.pc);
        self.pc = (self.pc + 1) & 0xFFFF;
        op
    }

    /// decode and execute one instruction
    pub fn step(&mut self) -> i32 {
        self.invalid_op = false;
        if self.enable_interrupt {
            self.iff1 = true;
            self.iff2 = true;
            self.enable_interrupt = false
        }
        return self.do_op(HL, 0);
    }

    /// compute effective address for (HL) or (IX/Y+d) instructions
    /// and update WZ register if needed
    fn addr(&mut self, m: usize, d: RegT) -> RegT {
        if m == HL {
            self.r16(HL)
        }
        else {
            self.wz = (self.r16(m) + d) & 0xFFFF;
            self.wz
        }
    }

    /// swap a 16-bit register with its counterpart
    fn swap16(&mut self, r : usize, r_ : usize) {
        let v = self.r16(r);
        let v_ = self.r16(r_);
        self.w16(r, v_);
        self.w16(r_, v);
    }

    /// execute a single 'main-instruction'
    ///
    /// This function may be called recursively for prefixed
    /// instructions
    ///
    /// * 'm'   - index of 16-bit register (may be HL, IX or IY)
    /// * 'd'   - the d in (IX+d), (IY+d), 0 if m is HL
    ///
    /// returns number of cycles the instruction takes
    pub fn do_op(&mut self, m: usize, d: RegT) -> i32 {
        let mut cyc = if m == HL { 0 } else { 4 };
        let ext_cyc = if m == HL { 0 } else { 8 };
        let op = self.fetch_op();

        // split instruction byte into bit groups
        let x = op>>6;
        let y = (op>>3 & 7) as usize;
        let z = (op & 7) as usize;
        
        //--- block 1: 8-bit loads
        if x == 1 {
            if y == 6 {
                if z == 6 {
                    // special case LD (HL),(HL) -> HALT
                    self.halt(); 
                    cyc += 4;
                }
                else {
                    // LD (HL),r; LD (IX+d),r; LD (IY+d),r
                    let a = self.addr(m, d);
                    self.mem.w8(a, self.reg[z]);
                    cyc += 7 + ext_cyc;
                }
            }
            else if z == 6 {
                // LD r,(HL); LD r,(IX+d); LD r,(IY+d)
                let a = self.addr(m, d);
                self.reg[y] = self.mem.r8(a);
                cyc += 7 + ext_cyc; 
            }
            else {
                // LD r,s
                self.reg[y] = self.reg[z];
                cyc += 4;
            }
        }

        //--- block 2: 8-bit ALU instructions
        else if x == 2 {
            let val = if z == 6 {
                // ALU (HL); ALU (IX+d); ALU (IY+d)
                cyc += 7 + ext_cyc;
                let a = self.addr(m, d);
                self.mem.r8(a)
            }
            else {
                // ALU r
                cyc += 4;
                self.reg[z]
            };
            match y {
                0 => self.add8(val),
                1 => self.adc8(val),
                2 => self.sub8(val),
                3 => self.sbc8(val),
                4 => self.and8(val),
                5 => self.xor8(val),
                6 => self.or8(val),
                7 => self.cp8(val),
                _ => (),
            }
        }
        
        //--- block 0: misc ops
        else if x == 0 {
            if z == 0 {
                match y {
                    // NOP
                    0 => { cyc += 4 },
                    // EX AF,AF'
                    1 => { self.swap16(AF, AF_); cyc += 4; },
                    // DJNZ
                    2 => {
                        self.reg[B] = (self.reg[B] - 1) & 0xFF;
                        if self.reg[B] > 0 {
                            let d = self.mem.rs8(self.pc);
                            self.wz = (self.pc + d + 1) & 0xFFFF;
                            self.pc = self.wz;
                            cyc += 13;
                        }
                        else {
                            self.pc = (self.pc + 1) & 0xFFFF;
                            cyc += 8;
                        }
                    },
                    _ => ()
                }
            }
            else if z == 1 {

            }
            else if z == 2 {

            }
            else if z == 3 {

            }
            else if z == 4 {
                if y == 6 {
                    // INC (HL); INC (IX+d); INC (IY+d)
                    let a = self.addr(m, d);
                    let v = self.mem.r8(a);
                    let w = self.inc8(v);
                    self.mem.w8(a, w);
                    cyc += 11 + ext_cyc;
                }
                else {
                    // INC r
                    let v = self.reg[y];
                    self.reg[y] = self.inc8(v);
                    cyc += 4;
                }
            }
            else if z == 5 {
                if y == 6 {
                    // DEC (HL); DEC (IX+d); DEC (IY+d)
                    let a = self.addr(m, d);
                    let v = self.mem.r8(a);
                    let w = self.dec8(v);
                    self.mem.w8(a, w);
                    cyc += 11 + ext_cyc;
                }
                else {
                    // DEC r
                    let v = self.reg[y];
                    self.reg[y] = self.dec8(v);
                    cyc += 4;
                }
            }
            else if z == 6 {
                let v = self.mem.r8(self.pc);
                self.pc = (self.pc + 1) & 0xFFFF;
                if y == 6 {
                    // LD (HL),n; LD (IX+d),n; LD (IY+d),n
                    let a = self.addr(m, d);
                    self.mem.w8(a, v);
                    cyc += if m == HL { 10 } else { 15 };
                }
                else {
                    self.reg[y] = v;
                    cyc += 7;
                }
            }
        }

        // return resulting number of CPU cycles taken
        cyc
    }

    pub fn halt(&mut self) {
        self.halt = true;
        self.pc -= 1;
    }

    pub fn push(&mut self, val: RegT) {
        let sp = self.r16(SP).wrapping_sub(2);
        self.w16(SP, sp);
        self.mem.w16(sp, val);
    }

    pub fn rst(&mut self, val: RegT) {
        // workaround for https://github.com/rust-lang/rust/issues/29975
        let pc = self.pc;
        self.push(pc);
        self.pc = val;
        self.wz = self.pc;
    }

    pub fn flags_add(acc: RegT, add: RegT, res: RegT) -> RegT {
        (if (res & 0xFF)==0 { ZF } else { res & SF }) |
        (res & (YF|XF)) |
        ((res>>8) & CF) |            
        ((acc^add^res) & HF) |
        ((((acc^add^0x80) & (add^res))>>5) & VF)
    }

    pub fn flags_sub(acc: RegT, sub: RegT, res: RegT) -> RegT {
        NF | 
        (if (res & 0xFF)==0 { ZF } else { res & SF }) |
        (res & (YF|XF)) |
        ((res>>8) & CF) |            
        ((acc^sub^res) & HF) |
        ((((acc^sub) & (res^acc))>>5) & VF)
    }

    pub fn flags_cp(acc: RegT, sub: RegT, res: RegT) -> RegT {
        // the only difference to flags_sub() is that the 
        // 2 undocumented flag bits X and Y are taken from the
        // sub-value, not the result
        NF | 
        (if (res & 0xFF)==0 { ZF } else { res & SF }) |
        (sub & (YF|XF)) |
        ((res>>8) & CF) |            
        ((acc^sub^res) & HF) |
        ((((acc^sub) & (res^acc))>>5) & VF)
    }

    pub fn flags_szp(val: RegT) -> RegT {
        let v = val & 0xFF;
        (if (v.count_ones()&1)==0 { PF } else { 0 }) |
        (if v==0 { ZF } else { v & SF }) |
        (v & (YF|XF))
    }

    pub fn add8(&mut self, add: RegT) {
        let acc = self.reg[A];
        let res = acc + add;
        self.reg[F] = CPU::flags_add(acc, add, res);
        self.reg[A] = res & 0xFF;
    }

    pub fn adc8(&mut self, add: RegT) {
        let acc = self.reg[A];
        let res = acc + add + (self.reg[F] & CF);
        self.reg[F] = CPU::flags_add(acc, add, res);
        self.reg[A] = res & 0xFF;
    }

    pub fn sub8(&mut self, sub: RegT) {
        let acc = self.reg[A];
        let res = acc - sub;
        self.reg[F] = CPU::flags_sub(acc, sub, res);
        self.reg[A] = res & 0xFF;
    }

    pub fn sbc8(&mut self, sub: RegT) {
        let acc = self.reg[A];
        let res = acc - sub - (self.reg[F] & CF);
        self.reg[F] = CPU::flags_sub(acc, sub, res);
        self.reg[A] = res & 0xFF;
    }

    pub fn cp8(&mut self, sub: RegT) {
        let acc = self.reg[A];
        let res = acc - sub;
        self.reg[F] = CPU::flags_cp(acc, sub, res);
    }

    pub fn neg8(&mut self) {
        let sub = self.reg[A];
        self.reg[A] = 0;
        self.sub8(sub);
    }

    pub fn and8(&mut self, val: RegT) {
        self.reg[A] &= val;
        self.reg[F] = CPU::flags_szp(self.reg[A])|HF;
    }

    pub fn or8(&mut self, val: RegT) {
        self.reg[A] |= val;
        self.reg[F] = CPU::flags_szp(self.reg[A]);
    }

    pub fn xor8(&mut self, val: RegT) {
        self.reg[A] ^= val;
        self.reg[F] = CPU::flags_szp(self.reg[A]);
    }

    pub fn inc8(&mut self, val: RegT) -> RegT {
        let res = (val + 1) & 0xFF;
        self.reg[F] = (if res==0 {ZF} else {res & SF}) |
            (res & (XF|YF)) | 
            ((res^val) & HF) |
            (if res==0x80 {VF} else {0}) |
            (self.reg[F] & CF);
        res
    }

    pub fn dec8(&mut self, val: RegT) -> RegT {
        let res = (val - 1) & 0xFF;
        self.reg[F] = NF | 
            (if res==0 {ZF} else {res & SF}) |
            (res & (XF|YF)) |
            ((res^val) & HF) |
            (if res==0x7F {VF} else {0}) |
            (self.reg[F] & CF);
        res
    }

    pub fn rlc8(&mut self, val: RegT) -> RegT {
        let res = (val<<1 | val>>7) & 0xFF;
        self.reg[F] = CPU::flags_szp(res) | ((val>>7) & CF);
        res
    }

    pub fn rlca8(&mut self) {
        let acc = self.reg[A];
        let res = (acc<<1 | acc>>7) & 0xFF;
        self.reg[F] = ((acc>>7) & CF) | (res & (XF|YF)) | (self.reg[F] & (SF|ZF|PF));
        self.reg[A] = res;
    }

    pub fn rrc8(&mut self, val: RegT) -> RegT {
        let res = (val>>1 | val<<7) & 0xFF;
        self.reg[F] = CPU::flags_szp(res) | (val & CF);
        res
    }

    pub fn rrca8(&mut self) {
        let acc = self.reg[A];
        let res = (acc>>1 | acc<<7) & 0xFF;
        self.reg[F] = (acc & CF) | (res & (XF|YF)) | (self.reg[F] & (SF|ZF|PF));
        self.reg[A] = res;
    }

    pub fn rl8(&mut self, val: RegT) -> RegT {
        let res = (val<<1 | (self.reg[F] & CF)) & 0xFF;
        self.reg[F] = CPU::flags_szp(res) | ((val>>7) & CF);
        res
    }

    pub fn rla8(&mut self) {
        let acc = self.reg[A];
        let res = (acc<<1 | (self.reg[F] & CF)) & 0xFF;
        self.reg[F] = ((acc>>7) & CF) | (res & (XF|YF)) | (self.reg[F] & (SF|ZF|PF));
        self.reg[A] = res;
    }

    pub fn rr8(&mut self, val: RegT) -> RegT {
        let res = (val>>1 | (self.reg[F] & CF)<<7) & 0xFF;
        self.reg[F] = CPU::flags_szp(res) | (val & CF);
        res
    }

    pub fn rra8(&mut self) {
        let acc = self.reg[A];
        let res = (acc>>1 | (self.reg[F] & CF)<<7) & 0xFF;
        self.reg[F] = (acc & CF) | (res & (XF|YF)) | (self.reg[F] & (SF|ZF|PF));
        self.reg[A] = res
    }

    pub fn sla8(&mut self, val: RegT) -> RegT {
        let res = (val<<1) & 0xFF;
        self.reg[F] = CPU::flags_szp(res) | (val>>7 & CF);
        res
    }

    pub fn sll8(&mut self, val: RegT) -> RegT {
        // undocumented, sll8 is identical with sla8, but shifts a 1 into LSB
        let res = (val<<1 | 1) & 0xFF;
        self.reg[F] = CPU::flags_szp(res) | (val>>7 & CF);
        res
    }

    pub fn sra8(&mut self, val: RegT) -> RegT {
        let res = (val>>1 | (val & 0x80)) & 0xFF;
        self.reg[F] = CPU::flags_szp(res) | (val & CF);
        res
    }

    pub fn srl8(&mut self, val: RegT) -> RegT {
        let res = val>>1 & 0xFF;
        self.reg[F] = CPU::flags_szp(res) | (val & CF);
        res
    }
    
    pub fn bit(&mut self, val: RegT, mask: RegT) {
        let res = val & mask;
        self.reg[F] = HF | (self.reg[F] & CF) |
            (if res == 0 {ZF|PF} else {res & SF}) |
            (val & (XF|YF));
    }

    pub fn ibit(&mut self, val: RegT, mask: RegT) {
        // special version of the BIT instruction for 
        // (HL), (IX+d), (IY+d) to set the undocumented XF|YF flags
        // from high byte of HL+1 or IX/IY+d (expected in WZ)
        let res = val & mask;
        self.reg[F] = HF | (self.reg[F] & CF) |
            (if res == 0 {ZF|PF} else {res & SF}) |
            ((self.wz >> 8) & (XF|YF));
    }

    pub fn add16(&mut self, acc: RegT, add: RegT) -> RegT {
        self.wz = (acc + 1) & 0xFFFF;
        let res = acc + add;
        self.reg[F] = (self.reg[F] & (SF|ZF|VF)) |
            (((acc^res^add)>>8) & HF) |
            (res>>16 & CF) | (res>>8 & (YF|XF));
        res & 0xFFFF
    }
    
    pub fn adc16(&mut self, acc: RegT, add: RegT) -> RegT {
        self.wz = (acc + 1) & 0xFFFF;
        let res = acc + add + (self.reg[F] & CF);
        self.reg[F] = (((acc^res^add)>>8) & HF) |
            ((res>>16) & CF) |
            ((res>>8) & (SF|XF|YF)) |
            (if (res & 0xFFFF) == 0 {ZF} else {0}) |
            (((add^acc^0x8000) & (add^res) & 0x8000)>>13);
        res & 0xFFFF
    }

    pub fn sbc16(&mut self, acc: RegT, sub: RegT) -> RegT {
        self.wz = (acc + 1) & 0xFFFF;
        let res = acc - sub - (self.reg[F] & CF);
        self.reg[F] = NF | (((acc^res^sub)>>8) & HF) |
            ((res>>16) & CF) |
            ((res>>8) & (SF|XF|YF)) |
            (if (res & 0xFFFF) == 0 {ZF} else {0}) |
            (((sub^acc) & (acc^res) & 0x8000)>>13);
        res & 0xFFFF
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use RegT;

    #[test]
    fn new() {
        let cpu = CPU::new();
        for v in cpu.reg.iter() {
            assert!(*v == 0);
        }
        assert!(0 == cpu.wz);
        assert!(0 == cpu.wz_);
        assert!(0 == cpu.pc);
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

    fn test_flags(cpu: &CPU, expected: RegT) -> bool {
        (cpu.reg[F] & !(XF|YF)) == expected
    }

    #[test]
    fn add8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0xF;
        cpu.add8(0xF);
        assert!(0x1E == cpu.reg[A]);
        assert!(test_flags(&cpu, HF));
        cpu.add8(0xE0);
        assert!(0xFE == cpu.reg[A]);
        assert!(test_flags(&cpu, SF));
        cpu.reg[A] = 0x81;
        cpu.add8(0x80);
        assert!(0x01 == cpu.reg[A]);
        assert!(test_flags(&cpu, VF|CF));
        cpu.add8(0xFF);
        assert!(0x00 == cpu.reg[A]);
        assert!(test_flags(&cpu, ZF|HF|CF));
    }

    #[test]
    fn adc8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0x00;
        cpu.adc8(0x00);
        assert!(0x00 == cpu.reg[A]);
        assert!(test_flags(&cpu, ZF));
        cpu.adc8(0x41);
        assert!(0x41 == cpu.reg[A]);
        assert!(test_flags(&cpu, 0));
        cpu.adc8(0x61);
        assert!(0xA2 == cpu.reg[A]);
        assert!(test_flags(&cpu, SF|VF));
        cpu.adc8(0x81);
        assert!(0x23 == cpu.reg[A]);
        assert!(test_flags(&cpu, VF|CF));
        cpu.adc8(0x41);
        assert!(0x65 == cpu.reg[A]);
        assert!(test_flags(&cpu, 0));
    }

    #[test]
    fn sub8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0x04;
        cpu.sub8(0x04);
        assert!(0x00 == cpu.reg[A]);
        assert!(test_flags(&cpu, ZF|NF));
        cpu.sub8(0x01);
        assert!(0xFF == cpu.reg[A]);
        assert!(test_flags(&cpu, SF|HF|NF|CF));
        cpu.sub8(0xF8);
        assert!(0x07 == cpu.reg[A]);
        assert!(test_flags(&cpu, NF));
        cpu.sub8(0x0F);
        assert!(0xF8 == cpu.reg[A]);
        assert!(test_flags(&cpu, SF|HF|NF|CF));
    }

    #[test]
    fn sbc8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0x04;
        cpu.sbc8(0x04);
        assert!(0x00 == cpu.reg[A]);
        assert!(test_flags(&cpu, ZF|NF));
        cpu.sbc8(0x01);
        assert!(0xFF == cpu.reg[A]);
        assert!(test_flags(&cpu, SF|HF|NF|CF));
        cpu.sbc8(0xF8);
        assert!(0x06 == cpu.reg[A]);
        assert!(test_flags(&cpu, NF));
    }

    #[test]
    fn cp8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0x04;
        cpu.cp8(0x04);
        assert!(test_flags(&cpu, ZF|NF)); 
        cpu.cp8(0x05);
        assert!(test_flags(&cpu, SF|HF|NF|CF));
        cpu.cp8(0x03);
        assert!(test_flags(&cpu, NF));
        cpu.cp8(0xFF);
        assert!(test_flags(&cpu, HF|NF|CF));
    }

    #[test]
    fn neg8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0x01;
        cpu.neg8();
        assert!(0xFF == cpu.reg[A]);
        assert!(test_flags(&cpu, SF|HF|NF|CF));
        cpu.reg[A] = 0x00;
        cpu.neg8();
        assert!(0x00 == cpu.reg[A]);
        assert!(test_flags(&cpu, NF|ZF));
        cpu.reg[A] = 0x80;
        cpu.neg8();
        assert!(0x80 == cpu.reg[A]);
        assert!(test_flags(&cpu, SF|VF|NF|CF))
    }

    #[test]
    fn and8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0xFF; cpu.and8(0x01);
        assert!(0x01 == cpu.reg[A]); assert!(test_flags(&cpu, HF));
        cpu.reg[A] = 0xFF; cpu.and8(0xAA);
        assert!(0xAA == cpu.reg[A]); assert!(test_flags(&cpu, SF|HF|PF));
        cpu.reg[A] = 0xFF; cpu.and8(0x03);
        assert!(0x03 == cpu.reg[A]); assert!(test_flags(&cpu, HF|PF));
    }

    #[test]
    fn or8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0x00; 
        cpu.or8(0x00);
        assert!(0x00 == cpu.reg[A]); assert!(test_flags(&cpu, ZF|PF));
        cpu.or8(0x01);
        assert!(0x01 == cpu.reg[A]); assert!(test_flags(&cpu, 0));
        cpu.or8(0x02);
        assert!(0x03 == cpu.reg[A]); assert!(test_flags(&cpu, PF));
    }

    #[test]
    fn xor8() {
        let mut cpu = CPU::new();
        cpu.reg[A] = 0x00;
        cpu.xor8(0x00);
        assert!(0x00 == cpu.reg[A]); assert!(test_flags(&cpu, ZF|PF));
        cpu.xor8(0x01);
        assert!(0x01 == cpu.reg[A]); assert!(test_flags(&cpu, 0));
        cpu.xor8(0x03);
        assert!(0x02 == cpu.reg[A]); assert!(test_flags(&cpu, 0));
    }

    #[test]
    fn inc8_dec8() {
        let mut cpu = CPU::new();
        let a = cpu.inc8(0x00);
        assert!(0x01 == a); assert!(test_flags(&cpu, 0));
        let b = cpu.dec8(a);
        assert!(0x00 == b); assert!(test_flags(&cpu, ZF|NF));
        let c = cpu.inc8(0xFF);
        assert!(0x00 == c); assert!(test_flags(&cpu, ZF|HF));
        let d = cpu.dec8(c);
        cpu.reg[F] |= CF;   // set carry flag (should be preserved)
        assert!(0xFF == d); assert!(test_flags(&cpu, SF|HF|NF|CF));
        let e = cpu.inc8(0x0F);
        assert!(0x10 == e); assert!(test_flags(&cpu, HF|CF));
        let f = cpu.dec8(e);
        assert!(0x0F == f); assert!(test_flags(&cpu, HF|NF|CF));
    }

    #[test]
    fn rlc8_rrc8() {
        let mut cpu = CPU::new();
        let a = cpu.rrc8(0x01);
        assert!(0x80 == a); assert!(test_flags(&cpu, SF|CF));
        let b = cpu.rlc8(a);
        assert!(0x01 == b); assert!(test_flags(&cpu, CF));
        let c = cpu.rrc8(0xFF);
        assert!(0xFF == c); assert!(test_flags(&cpu, SF|PF|CF));
        let d = cpu.rlc8(c);
        assert!(0xFF == d); assert!(test_flags(&cpu, SF|PF|CF));
        let e = cpu.rlc8(0x03);
        assert!(0x06 == e); assert!(test_flags(&cpu, PF));
        let f = cpu.rrc8(e);
        assert!(0x03 == f); assert!(test_flags(&cpu, PF));
    }

    #[test]
    fn rlca8_rrca8() {
        let mut cpu = CPU::new();
        cpu.reg[F] = 0xFF;
        cpu.reg[A] = 0xA0;
        cpu.rlca8();
        assert!(0x41 == cpu.reg[A]); assert!(test_flags(&cpu, SF|ZF|VF|CF));
        cpu.rlca8();
        assert!(0x82 == cpu.reg[A]); assert!(test_flags(&cpu, SF|ZF|VF));
        cpu.rrca8();
        assert!(0x41 == cpu.reg[A]); assert!(test_flags(&cpu, SF|ZF|VF));
        cpu.rrca8();
        assert!(0xA0 == cpu.reg[A]); assert!(test_flags(&cpu, SF|ZF|VF|CF));
    }

    #[test]
    fn rl8_rr8() {
        let mut cpu = CPU::new();
        let a = cpu.rr8(0x01);
        assert!(0x00 == a); assert!(test_flags(&cpu, ZF|PF|CF));
        let b = cpu.rl8(a);
        assert!(0x01 == b); assert!(test_flags(&cpu, 0));
        let c = cpu.rr8(0xFF);
        assert!(0x7F == c); assert!(test_flags(&cpu, CF));
        let d = cpu.rl8(c);
        assert!(0xFF == d); assert!(test_flags(&cpu, SF|PF));
        let e = cpu.rl8(0x03);
        assert!(0x06 == e); assert!(test_flags(&cpu, PF));
        let f = cpu.rr8(e);
        assert!(0x03 == f); assert!(test_flags(&cpu, PF));
    }


    #[test]
    fn rla8_rra8() {
        let mut cpu = CPU::new();
        cpu.reg[F] = 0xFF;
        cpu.reg[A] = 0xA0;
        cpu.rla8();
        assert!(0x41 == cpu.reg[A]); assert!(test_flags(&cpu, SF|ZF|VF|CF));
        cpu.rla8();
        assert!(0x83 == cpu.reg[A]); assert!(test_flags(&cpu, SF|ZF|VF));
        cpu.rra8();
        assert!(0x41 == cpu.reg[A]); assert!(test_flags(&cpu, SF|ZF|VF|CF));
        cpu.rra8();
        assert!(0xA0 == cpu.reg[A]); assert!(test_flags(&cpu, SF|ZF|VF|CF));
    }

    #[test]
    fn sla8() {
        let mut cpu = CPU::new();
        let a = cpu.sla8(0x01);
        assert!(0x02 == a); assert!(test_flags(&cpu, 0));
        let b = cpu.sla8(0x80);
        assert!(0x00 == b); assert!(test_flags(&cpu, ZF|PF|CF));
        let c = cpu.sla8(0xAA);
        assert!(0x54 == c); assert!(test_flags(&cpu, CF));
        let d = cpu.sla8(0xFE);
        assert!(0xFC == d); assert!(test_flags(&cpu, SF|PF|CF));
        let e = cpu.sla8(0x7F);
        assert!(0xFE == e); assert!(test_flags(&cpu, SF));
    }

    #[test]
    fn sra8() {
        let mut cpu = CPU::new();
        let a = cpu.sra8(0x01);
        assert!(0x00 == a); assert!(test_flags(&cpu, ZF|PF|CF));
        let b = cpu.sra8(0x80);
        assert!(0xC0 == b); assert!(test_flags(&cpu, SF|PF));
        let c = cpu.sra8(0xAA);
        assert!(0xD5 == c); assert!(test_flags(&cpu, SF));
        let d = cpu.sra8(0xFE);
        assert!(0xFF == d); assert!(test_flags(&cpu, SF|PF));
    }

    #[test]
    fn srl8() {
        let mut cpu = CPU::new();
        let a = cpu.srl8(0x01);
        assert!(0x00 == a); assert!(test_flags(&cpu, ZF|PF|CF));
        let b = cpu.srl8(0x80);
        assert!(0x40 == b); assert!(test_flags(&cpu, 0));
        let c = cpu.srl8(0xAA);
        assert!(0x55 == c); assert!(test_flags(&cpu, PF));
        let d = cpu.srl8(0xFE);
        assert!(0x7f == d); assert!(test_flags(&cpu, 0));
        let e = cpu.srl8(0x7F);
        assert!(0x3F == e); assert!(test_flags(&cpu, PF|CF));
    }
}

