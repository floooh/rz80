use std::mem;

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
pub const A : usize = 6;
pub const F : usize = 7;
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

    enable_interrupt: bool,
    m_r  : [usize; 8],
    m_r2 : [usize; 8],
    m_sp : [usize; 4],
    m_af : [usize; 4],
    
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
            m_r  : [ B, C, D, E, H, L, F, A ],
            m_r2 : [ B, C, D, E, H, L, F, A ],
            m_sp : [ BC, DE, HL, SP ],
            m_af : [ BC, DE, HL, AF ],
            mem: Memory::new(),

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

    /// write 8-bit register value with register mapping through m_r[]
    pub fn w8(&mut self, r: usize, val: RegT) {
        self.reg[self.m_r[r]] = val;
    }

    /// read 8-bit register value with register mapping though m_r[]
    pub fn r8(&self, r: usize) -> RegT {
        self.reg[self.m_r[r]]
    }

    /// write 16-bit register value with direct index
    pub fn w16_i(&mut self, i: usize, val: RegT) {
        self.reg[i]   = (val>>8) & 0xFF;
        self.reg[i+1] = val & 0xFF; 
    }

    /// read 16-bit register value with direct index
    pub fn r16_i(&self, i: usize) -> RegT {
        (self.reg[i]<<8) | self.reg[i+1]
    }

    /// write 16-bit register value with mapping through m_sp[]
    pub fn w16_sp(&mut self, r: usize, val: RegT) {
        let i = self.m_sp[r];
        self.w16_i(i, val);
    }

    /// read 16-bit register value with mapping through m_sp[]
    pub fn r16_sp(&self, r: usize) -> RegT {
        self.r16_i(self.m_sp[r])
    }

    /// write 16-bit register value with mapping through m_af[]
    pub fn w16_af(&mut self, r: usize, val: RegT) {
        let i = self.m_af[r];
        self.w16_i(i, val);
    }

    /// read 16-bit register value with mapping through m_af[]
    pub fn r16_af(&self, r: usize) -> RegT {
        self.r16_i(self.m_af[r])
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
        return self.do_op(false);
    }

    /// load 8-bit unsigned immediate operand and increment PC
    fn imm8(&mut self) -> RegT {
        let imm = self.mem.r8(self.pc);
        self.pc = (self.pc + 1) & 0xFFFF;
        imm
    }

    /// load 16-bit immediate operand and bump PC
    fn imm16(&mut self) -> RegT {
        let imm = self.mem.r16(self.pc);
        self.pc = (self.pc + 2) & 0xFFFF;
        imm
    }

    /// compute effective address for (HL) or (IX/Y+d) instructions
    /// and update WZ register if needed
    fn addr(&mut self, ext: bool) -> RegT {
        if ext {
            let d = self.mem.rs8(self.pc);
            self.pc = (self.pc + 1) & 0xFFFF;
            self.wz = (self.r16_sp(2) + d) & 0xFFFF;
            self.wz
        }
        else {
            self.r16_sp(2)
        }
    }

    /// swap a 16-bit register with its counterpart
    fn swap16(&mut self, r : usize, r_ : usize) {
        let v = self.r16_i(r);
        let v_ = self.r16_i(r_);
        self.w16_i(r, v_);
        self.w16_i(r_, v);
    }

    /// check condition (for conditional jumps etc)
    fn cc(&self, y: usize) -> bool {
        match y {
            0 => 0 == self.reg[F] & ZF, // JR NZ
            1 => 0 != self.reg[F] & ZF, // JR Z
            2 => 0 == self.reg[F] & CF, // JR NC
            3 => 0 != self.reg[F] & CF, // JC C
            4 => 0 == self.reg[F] & PF, // JR PO
            5 => 0 != self.reg[F] & PF, // JR PE
            6 => 0 == self.reg[F] & SF, // JR P
            7 => 0 != self.reg[F] & SF, // JR M
            _ => false,
        }
    }

    /// patch register mapping tables for DD/FD extended instructions
    fn patch_reg_tables(&mut self, rr: usize, rh: usize, rl: usize) {
        self.m_r[H] = rh;       // H replaced with IXH or IYH
        self.m_r[L] = rl;       // L replaced with IXL or IYL
        self.m_sp[2] = rr;     // HL replaced with IX or IY
        self.m_af[2] = rr;     // ditto
    }

    /// unpatch register mapping tables after extended instructions
    fn unpatch_reg_tables(&mut self) {
        self.m_r[H] = H;
        self.m_r[L] = L;
        self.m_sp[2] = HL;
        self.m_af[2] = HL;
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
    pub fn do_op(&mut self, ext: bool) -> i32 {
        let (mut cyc, ext_cyc) = if ext {(4,8)} else {(0,0)};
        let op = self.fetch_op();

        // split instruction byte into bit groups
        let x = op>>6;
        let y = (op>>3 & 7) as usize;
        let z = (op & 7) as usize;
        let p = y>>1;
        let q = y & 1;
        match (x, y, z) {
        //--- block 1: 8-bit loads
            // special case LD (HL),(HL): HALT
            (1, 6, 6) => {
                self.halt(); 
                cyc += 4;
            },
            // LD (HL),r; LD (IX+d),r; LD (IY+d),r
            // NOTE: this always loads from H,L, never IXH, ...
            (1, 6, _) => {
                let a = self.addr(ext);
                self.mem.w8(a, self.reg[self.m_r2[z]]);
                cyc += 7 + ext_cyc;
            },
            // LD r,(HL); LD r,(IX+d); LD r,(IY+d)
            // NOTE: this always loads to H,L, never IXH,...
            (1, _, 6) => {
                let a = self.addr(ext);
                self.reg[self.m_r2[y]] = self.mem.r8(a);
                cyc += 7 + ext_cyc; 
            },
            // LD r,s
            (1, _, _) => {
                let v = self.r8(z);
                self.w8(y, v);
                cyc += 4;
            },
        //--- block 2: 8-bit ALU instructions
            // ALU (HL); ALU (IX+d); ALU (IY+d)
            (2, _, _) => {
                let val = if z == 6 {
                    // ALU (HL); ALU (IX+d); ALU (IY+d)
                    cyc += 7 + ext_cyc;
                    let a = self.addr(ext);
                    self.mem.r8(a)
                }
                else {
                    // ALU r
                    cyc += 4;
                    self.r8(z)
                };
                self.alu8(y, val);
            },
        //--- block 0: misc ops
            // NOP
            (0, 0, 0) => { 
                cyc += 4 
            },
            // EX AF,AF'
            (0, 1, 0) => { 
                self.swap16(AF, AF_); cyc += 4; 
            },
            // DJNZ
            (0, 2, 0) => { 
                cyc += self.djnz(); 
            },
            // JR d
            (0, 3, 0) => {
                self.wz = (self.pc + self.mem.rs8(self.pc) + 1) & 0xFFFF;
                self.pc = self.wz;
                cyc += 12;
            },
            // JR cc
            (0, _, 0) => {
                if self.cc(y-4) {
                    self.wz = (self.pc + self.mem.rs8(self.pc) + 1) & 0xFFFF;
                    self.pc = self.wz;
                    cyc += 12;
                }
                else {
                    self.pc = (self.pc + 1) & 0xFFFF;
                    cyc += 7;                    
                }
            }
            // 16-bit immediate loads and 16-bit ADD
            (0, _, 1) => {
                if q == 0 {
                    // LD rr,nn (inkl IX,IY)
                    let val = self.imm16();
                    self.w16_sp(p, val);
                    cyc += 10;
                }
                else {
                    // ADD HL,rr; ADD IX,rr; ADD IY,rr
                    let acc = self.r16_sp(2);
                    let val = self.r16_sp(p);
                    let res = self.add16(acc, val);
                    self.w16_sp(2, res);
                    cyc += 11;
                }
            },
            (0, _, 2) => {
                // indirect loads
                match (q, p) {
                    // LD (nn),HL; LD (nn),IX; LD (nn),IY
                    (0, 2) => {
                        let addr = self.imm16();
                        let v = self.r16_sp(2);
                        self.mem.w16(addr, v);
                        self.wz = (addr + 1) & 0xFFFF;
                        cyc += 16;
                    },
                    // LD (nn),A
                    (0, 3) => {
                        let addr = self.imm16();
                        self.mem.w8(addr, self.reg[A]);
                        self.wz = (addr + 1) & 0xFFFF;
                        cyc += 13;
                    }
                    // LD (BC),A; LD (DE),A,; LD (nn),A
                    (0, _) => {
                        let addr = if p==0 { self.r16_i(BC) } else { self.r16_i(DE) };
                        self.mem.w8(addr, self.reg[A]); 
                        self.wz = (self.reg[A]<<8) | ((addr+1) & 0xFF);
                        cyc += 7;
                    },
                    // LD HL,(nn); LD IX,(nn); LD IY,(nn)
                    (1, 2) => {
                        let addr = self.imm16();
                        let val  = self.mem.r16(addr);
                        self.w16_sp(2, val);
                        self.wz = (addr + 1) & 0xFFFF;
                        cyc += 16;
                    },
                    // LD A,(nn)
                    (1, 3) => {
                        let addr = self.imm16();
                        self.reg[A] = self.mem.r8(addr);
                        self.wz = (addr + 1) & 0xFFFF;
                        cyc += 13;
                    }
                    // LD A,(BC); LD A,(DE)
                    (1, _) => {
                        let addr = if p == 0 { self.r16_i(BC) } else { self.r16_i(DE) };
                        self.reg[A] = self.mem.r8(addr);
                        self.wz = (addr + 1) & 0xFFFF;
                        cyc += 7;
                    },
                    (_,_) => { }
                }
            },
            (0, _, 3) => {
                // 16-bit INC/DEC
                let val = (self.r16_sp(p) + if q==0 {1} else {-1}) & 0xFFFF;
                self.w16_sp(p, val);
                cyc += 6
            },
            // INC (HL); INC (IX+d); INC (IY+d)
            (0, 6, 4) => {
                let a = self.addr(ext);
                let v = self.mem.r8(a);
                let w = self.inc8(v);
                self.mem.w8(a, w);
                cyc += 11 + ext_cyc;
            },
            // INC r
            (0, _, 4) => {
                let v = self.r8(y);
                let w = self.inc8(v);
                self.w8(y, w);
                cyc += 4;
            },
            // DEC (HL); DEC (IX+d); DEC (IY+d)
            (0, 6, 5) => {
                let a = self.addr(ext);
                let v = self.mem.r8(a);
                let w = self.dec8(v);
                self.mem.w8(a, w);
                cyc += 11 + ext_cyc;
            },
            // DEC r
            (0, _, 5) => {
                let v = self.r8(y);
                let w = self.dec8(v);
                self.w8(y, w);
                cyc += 4;
            },
            // LD r,n; LD (HL),n; LD (IX+d),n; LD (IY+d),n
            (0, _, 6) => {
                if y == 6 {
                    // LD (HL),n; LD (IX+d),n; LD (IY+d),n
                    let a = self.addr(ext);
                    let v = self.imm8();
                    self.mem.w8(a, v);
                    cyc += if ext { 15 } else { 10 };
                }
                else {
                    // LD r,n
                    let v = self.imm8();
                    self.w8(y, v);
                    cyc += 7;
                }
            },
            // misc ops on A and F
            (0, _, 7) => {
                match y {
                    0 => self.rlca8(),
                    1 => self.rrca8(),
                    2 => self.rla8(),
                    3 => self.rra8(),
                    4 => self.daa(),
                    5 => self.cpl(),
                    6 => self.scf(),
                    7 => self.ccf(),
                    _ => panic!("CAN'T HAPPEN!"),
                }
                cyc += 4;
            }
        //--- block 3: misc and prefixed ops
            (3, _, 0) => { 
                // RET cc
                cyc += self.retcc(y); 
            }
            (3, _, 1) => {
                match (q,p) {
                    (0, _) => {
                        // POP BC,DE,HL,IX,IY
                        let val = self.pop();
                        self.w16_af(p, val);
                        cyc += 10;
                    },
                    (1, 0) => {
                        // RET
                        cyc += self.ret();
                    },
                    (1, 1) => {
                        // EXX
                        self.swap16(BC, BC_);
                        self.swap16(DE, DE_);
                        self.swap16(HL, HL_);
                        mem::swap(&mut self.wz, &mut self.wz_);
                        cyc += 4;
                    },
                    (1, 2) => {
                        // JP HL; JP IX; JP IY
                        self.pc = self.r16_sp(2);
                        cyc += 4;
                    },
                    (1, 3) => {
                        // LD SP,HL, LD SP,IX; LD SP,IY
                        let v = self.r16_sp(2);
                        self.w16_i(SP, v);
                        cyc += 6;
                    },
                    (_, _) => {
                        panic!("Can't happen!");
                    }
                }
            },
            (3, _, 2) => {
                // JP cc,nn
                self.wz = self.imm16();
                if self.cc(y) {
                    self.pc = self.wz;
                }
                cyc += 10;
            },
            (3, _, 3) => {
                // misc ops
                match y {
                    0 => { 
                        self.wz = self.imm16();
                        self.pc = self.wz;
                        cyc += 10;
                    },
                    1 => {
                        panic!("FIXME: CB prefix");
                    },
                    2 => {
                        panic!("FIXME: OUT");
                    },
                    3 => {
                        panic!("FIXME IN");
                    },
                    4 => {
                        // EX (SP),HL; EX (SP),IX; EX (SP),IY
                        let sp = self.r16_i(SP);
                        let v  = self.r16_sp(2);
                        self.wz = self.mem.r16(sp);
                        self.mem.w16(sp, v);
                        let wz = self.wz;
                        self.w16_sp(2, wz);
                        cyc += 19;
                    },
                    5 => {
                        // EX DE,HL
                        self.swap16(DE, HL);
                        cyc += 4;
                    },
                    6 => {
                        // DI
                        self.iff1 = false; self.iff2 = false;
                        cyc += 4;
                    },
                    7 => {
                        // EI
                        self.enable_interrupt = true;
                    },
                    _ => panic!("Can't happen!")
                }
            },
            (3, _, 4) => {
                // CALL cc
                cyc += self.callcc(y);
            },
            (3, _, 5) => {
                match (q, p) {
                    (0, _) => {
                        // PUSH BC,DE,HL,IX,IY,AF
                        let v = self.r16_af(p);
                        self.push(v);
                        cyc += 11;
                    },
                    (1, 0) => {
                        // CALL nn
                        cyc += self.call();
                    },
                    (1, 1) => {
                        // DD prefix instructions
                        self.patch_reg_tables(IX, IXH, IXL);
                        cyc += self.do_op(true);
                        self.unpatch_reg_tables();
                    },
                    (1, 2) => {
                        // ED prefix instructions
                        cyc += self.do_ed_op();
                    },
                    (1, 3) => {
                        // FD prefix instructions
                        self.patch_reg_tables(IY, IYH, IYL);
                        cyc += self.do_op(true); 
                        self.unpatch_reg_tables();
                    },
                    (_, _) => {
                        panic!("Can't happen!");
                    }
                }
            },
            // ALU n
            (3, _, 6) => {
                let val = self.imm8();
                self.alu8(y, val);
                cyc += 7;
            },
            // RST
            (3, _, 7) => {
                self.rst((y * 8) as RegT);
                cyc += 11;
            },
            // not implemented
            _ => {
                panic!("Unimplemented Z80 instruction!");
            }
        }

        // return resulting number of CPU cycles taken
        cyc
    }

    /// fetch and execute ED prefix instruction
    fn do_ed_op(&mut self) -> i32 {
        let op = self.fetch_op();

        // split instruction byte into bit groups
        let x = op>>6;
        let y = (op>>3 & 7) as usize;
        let z = (op & 7) as usize;
//        let p = y>>1;
//        let q = y & 1;
        match (x, y, z) {
            // block instructions
            (2, 4, 0) => { self.ldi(); 16 },
            (2, 5, 0) => { self.ldd(); 16 },
            (2, 6, 0) => { self.ldir() },
            (2, 7, 0) => { self.lddr() },
            (2, 4, 1) => { self.cpi(); 16 },
            (2, 5, 1) => { self.cpd(); 16 },
            (2, 6, 1) => { self.cpir() },
            (2, 7, 1) => { self.cpdr() },
            (2, 4, 2) => { self.ini(); 16 },
            (2, 5, 2) => { self.ind(); 16 },
            (2, 6, 2) => { self.inir() },
            (2, 7, 2) => { self.indr() },
            (2, 4, 3) => { self.outi(); 16 },
            (2, 5, 3) => { self.outd(); 16 },
            (2, 6, 3) => { self.otir() },
            (2, 7, 3) => { self.otdr() },
            _ => panic!("FIXME!")
        }
    }

    pub fn halt(&mut self) {
        self.halt = true;
        self.pc -= 1;
    }

    pub fn push(&mut self, val: RegT) {
        let sp = (self.r16_i(SP) - 2) & 0xFFFF;
        self.w16_i(SP, sp);
        self.mem.w16(sp, val);
    }

    pub fn pop(&mut self) -> RegT {
        let sp = self.r16_i(SP);
        let val = self.mem.r16(sp);
        self.w16_i(SP, sp + 2);
        val
    }

    pub fn rst(&mut self, val: RegT) {
        let pc = self.pc;
        self.push(pc);
        self.pc = val;
        self.wz = self.pc;
    }

    pub fn alu8(&mut self, alu: usize, val: RegT) {
        match alu {
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

    pub fn djnz(&mut self) -> i32 {
        self.reg[B] = (self.reg[B] - 1) & 0xFF;
        if self.reg[B] > 0 {
            let d = self.mem.rs8(self.pc);
            self.wz = (self.pc + d + 1) & 0xFFFF;
            self.pc = self.wz;
            13  // return num cycles if branch taken
        }
        else {
            self.pc = (self.pc + 1) & 0xFFFF;
            8   // return num cycles if loop finished
        }
    }

    pub fn daa(&mut self) {
        let a = self.reg[A];
        let mut val = a;
        let f = self.reg[F];
        if 0 != (f & NF) {
            if ((a & 0xF) > 0x9) || (0 != (f & HF)) {
                val = (val - 0x06) & 0xFF;
            }
            if (a > 0x99) || (0 != (f & CF)) {
                val = (val - 0x60) & 0xFF;
            }
        }
        else {
            if ((a & 0xF) > 0x9) || (0 != (f & HF)) {
                val = (val + 0x06) & 0xFF;
            }
            if (a > 0x99) || (0 != (f & CF)) {
                val = (val + 0x60) & 0xFF;
            }
        }
        self.reg[F] = (f & (CF|NF)) |
            (if a>0x99 {CF} else {0}) |
            ((a^val) & HF) |
            CPU::flags_szp(val);
        self.reg[A] = val;
    }

    pub fn cpl(&mut self) {
        let f = self.reg[F];
        let a = self.reg[A] ^ 0xFF;
        self.reg[F] = (f & (SF|ZF|PF|CF)) | (HF|NF) | (a & (YF|XF));
        self.reg[A] = a;
    }

    pub fn scf(&mut self) {
        let f = self.reg[F];
        let a = self.reg[A];
        self.reg[F] = (f & (SF|ZF|YF|XF|PF)) | CF | (a & (YF|XF));
    }

    pub fn ccf(&mut self) {
        let f = self.reg[F];
        let a = self.reg[A];
        self.reg[F] = ((f & (SF|ZF|YF|XF|PF|CF)) | ((f & CF)<<4) | (a & (YF|XF))) ^ CF;
    }

    pub fn ret(&mut self) -> i32 {
        self.wz = self.mem.r16(self.r16_i(SP));
        self.pc = self.wz;
        let sp = self.r16_i(SP);
        self.w16_i(SP, sp + 2);
        10
    }

    pub fn call(&mut self) -> i32 {
        self.wz = self.imm16();
        let sp = (self.r16_i(SP) - 2) & 0xFFFF;
        self.mem.w16(sp, self.pc);
        self.w16_i(SP, sp);
        self.pc = self.wz;
        17 
    }

    pub fn retcc(&mut self, y: usize) -> i32 {
        if self.cc(y) {
            self.ret() + 1
        }
        else {
            5
        }
    }

    pub fn callcc(&mut self, y: usize) -> i32 {
        if self.cc(y) {
            self.call()
        }
        else {
            self.wz = self.imm16();
            10
        }               
    }

    pub fn ldi(&mut self) {
        let hl = self.r16_i(HL);
        let de = self.r16_i(DE);
        let val = self.mem.r8(hl);
        self.mem.w8(de, val);
        self.w16_i(HL, hl + 1);
        self.w16_i(DE, de + 1);
        let bc = (self.r16_i(BC) - 1) & 0xFFFF;
        self.w16_i(BC, bc);
        let n = (val + self.reg[A]) & 0xFF;
        self.reg[F] = (self.reg[F] & (SF|ZF|CF)) |
            (if (n & 0x02) != 0 {YF} else {0}) |
            (if (n & 0x08) != 0 {XF} else {0}) |
            (if bc > 0 {VF} else {0});
    }

    pub fn ldd(&mut self) {
        let hl = self.r16_i(HL);
        let de = self.r16_i(DE);
        let val = self.mem.r8(hl);
        self.mem.w8(de, val);
        self.w16_i(HL, hl - 1);
        self.w16_i(DE, de - 1);
        let bc = (self.r16_i(BC) - 1) & 0xFFFF;
        self.w16_i(BC, bc);
        let n = (val + self.reg[A]) & 0xFF;
        self.reg[F] = (self.reg[F] & (SF|ZF|CF)) |
            (if (n & 0x02) != 0 {YF} else {0}) |
            (if (n & 0x08) != 0 {XF} else {0}) |
            (if bc > 0 {VF} else {0});
    }

    pub fn ldir(&mut self) -> i32 {
        self.ldi();
        if (self.reg[F] & VF) != 0 {
            self.pc = (self.pc - 2) & 0xFFFF;
            self.wz = (self.pc + 1) & 0xFFFF;
            21
        }
        else {
            16
        }
    }

    pub fn lddr(&mut self) -> i32 {
        self.ldd();
        if (self.reg[F] & VF) != 0 {
            self.pc = (self.pc - 2) & 0xFFFF;
            self.wz = (self.pc + 1) & 0xFFFF;
            21
        }
        else {
            16
        }
    }

    pub fn cpi(&mut self) {
        panic!("FIXME: cpi!");
    }

    pub fn cpd(&mut self) {
        panic!("FIXME: cpd!");
    }

    pub fn cpir(&mut self) -> i32 {
        panic!("FIXME: cpir!");
    }

    pub fn cpdr(&mut self) -> i32 {
        panic!("FIXME: cpdr!");
    }

    pub fn ini(&mut self) {
        panic!("FIXME: ini!");
    }

    pub fn ind(&mut self) {
        panic!("FIXME: ind!");
    }

    pub fn inir(&mut self) -> i32 {
        panic!("FIXME: inir!");
    }

    pub fn indr(&mut self) -> i32 {
        panic!("FIXME: indr!");
    }

    pub fn outi(&mut self) {
        panic!("FIXME: outi!");
    }

    pub fn outd(&mut self) {
        panic!("FIXME: outd!");
    }

    pub fn otir(&mut self) -> i32 {
        panic!("FIXME: otir!");
    }

    pub fn otdr(&mut self) -> i32 {
        panic!("FIXME: otdr!");
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
        cpu.w16_i(BC, 0x1234);
        cpu.w16_i(DE, 0x5678);
        cpu.w16_i(HL, 0x1357);
        cpu.w16_i(AF, 0x1122);
        assert!(0x12 == cpu.reg[B]);
        assert!(0x34 == cpu.reg[C]);
        assert!(0x1234 == cpu.r16_i(BC));
        assert!(0x56 == cpu.reg[D]);
        assert!(0x78 == cpu.reg[E]);
        assert!(0x5678 == cpu.r16_i(DE));
        assert!(0x13 == cpu.reg[H]);
        assert!(0x57 == cpu.reg[L]);
        assert!(0x1357 == cpu.r16_i(HL));
        assert!(0x22 == cpu.reg[F]);
        assert!(0x11 == cpu.reg[A]);
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
        cpu.w16_i(SP, 0x100);
        cpu.rst(0x38);
        assert!(0xFE == cpu.r16_i(SP));
        assert!(cpu.mem.r16(cpu.r16_i(SP)) == 0x123);
        assert!(0x38 == cpu.pc);
        assert!(0x38 == cpu.wz);
    }

    #[test]
    fn push() {
        let mut cpu = CPU::new();
        cpu.w16_i(SP, 0x100);
        cpu.push(0x1234);
        assert!(0xFE == cpu.r16_i(SP));
        assert!(cpu.mem.r16(cpu.r16_i(SP)) == 0x1234);
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

