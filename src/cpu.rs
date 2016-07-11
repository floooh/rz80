use RegT;
use memory::Memory;
use registers::Registers;

/// the Z80 CPU state
pub struct CPU<I, O>
    where I: FnMut(RegT) -> RegT,
          O: FnMut(RegT, RegT) {

    pub reg : Registers,
    pub halt : bool,
    pub iff1: bool,
    pub iff2: bool,
    pub invalid_op: bool,
    enable_interrupt: bool,

    in_fn : I,
    out_fn : O,

    pub mem : Memory,
}

fn canthappen() -> ! {
    panic!("Can't happen!");
}

use registers::CF as CF;
use registers::NF as NF;
use registers::VF as VF;
use registers::PF as PF;
use registers::XF as XF;
use registers::HF as HF;
use registers::YF as YF;
use registers::ZF as ZF;
use registers::SF as SF;

#[inline(always)]
fn flags_add(acc: RegT, add: RegT, res: RegT) -> RegT {
    (if (res & 0xFF)==0 { ZF } else { res & SF }) |
    (res & (YF|XF)) |
    ((res>>8) & CF) |            
    ((acc^add^res) & HF) |
    ((((acc^add^0x80) & (add^res))>>5) & VF)
}

#[inline(always)]
fn flags_sub(acc: RegT, sub: RegT, res: RegT) -> RegT {
    NF | 
    (if (res & 0xFF)==0 { ZF } else { res & SF }) |
    (res & (YF|XF)) |
    ((res>>8) & CF) |            
    ((acc^sub^res) & HF) |
    ((((acc^sub) & (res^acc))>>5) & VF)
}

#[inline(always)]
fn flags_cp(acc: RegT, sub: RegT, res: RegT) -> RegT {
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

#[inline(always)]
fn flags_szp(val: RegT) -> RegT {
    let v = val & 0xFF;
    (if (v.count_ones()&1)==0 { PF } else { 0 }) |
    (if v==0 { ZF } else { v & SF }) |
    (v & (YF|XF))
}

#[inline(always)]
fn flags_sziff2(val: RegT, iff2: bool) -> RegT {
    (if (val & 0xFF)==0 {ZF} else {val & SF}) |
    (val & (YF|XF)) |
    if iff2 {PF} else {0}
}

use registers::BC as BC;
use registers::DE as DE;
use registers::HL as HL;
use registers::AF as AF;
use registers::WZ as WZ;
use registers::BC_ as BC_;
use registers::DE_ as DE_;
use registers::HL_ as HL_;
use registers::AF_ as AF_;
use registers::WZ_ as WZ_;

impl<I, O> CPU<I, O> 
    where I: FnMut(RegT) -> RegT,
          O: FnMut(RegT, RegT) {

    /// initialize a new Z80 CPU object with input/output callbacks
    pub fn new(in_func: I, out_func: O) -> CPU<I,O> {
        CPU {
            reg: Registers::new(),
            halt: false,
            iff1: false,
            iff2: false,
            invalid_op: false,
            enable_interrupt: false,
            in_fn : in_func,
            out_fn : out_func,
            mem: Memory::new(),

        }
    }
    /// reset the cpu
    pub fn reset(&mut self) {
        self.reg.reset();
        self.halt = false;
        self.iff1 = false;
        self.iff2 = false;
        self.invalid_op = false;
        self.enable_interrupt = false;
    }

    /// fetch the next instruction byte from memory
    #[inline(always)]
    fn fetch_op(&mut self) -> RegT {
        self.reg.r = (self.reg.r & 0x80) | ((self.reg.r+1) & 0x7F);
        let pc = self.reg.pc();
        let op = self.mem.r8(pc);
        self.reg.set_pc(pc + 1);
        op
    }

    /// decode and execute one instruction
    pub fn step(&mut self) -> i64 {
        self.invalid_op = false;
        if self.enable_interrupt {
            self.iff1 = true;
            self.iff2 = true;
            self.enable_interrupt = false
        }
        self.do_op(false)
    }

    /// load 8-bit unsigned immediate operand and increment PC
    #[inline(always)]
    fn imm8(&mut self) -> RegT {
        let pc  = self.reg.pc();
        let imm = self.mem.r8(pc);
        self.reg.set_pc(pc + 1);
        imm
    }

    /// load 16-bit immediate operand and bump PC
    #[inline(always)]
    fn imm16(&mut self) -> RegT {
        let pc  = self.reg.pc();
        let imm = self.mem.r16(pc);
        self.reg.set_pc(pc + 2);
        imm
    }

    /// load d (as in IX+d) from memory and advance PC
    #[inline(always)]
    fn d(&mut self) -> RegT {
        let pc = self.reg.pc();
        let d = self.mem.rs8(pc);
        self.reg.set_pc(pc + 1);
        d
    }

    /// load effective address HL, IX+d or IY+d with existing d
    /// this is for DD CB and FD DB instructions
    #[inline(always)]
    fn addr_d(&mut self, d: RegT, ext: bool) -> RegT {
        if ext {
            let addr = (self.reg.r16sp(2) + d) & 0xFFFF;
            self.reg.set_wz(addr);
            addr
        }
        else {
            self.reg.r16sp(2)
        }
    }

    /// load effective address for (HL) or (IX/Y+d) instructions
    /// and update WZ register if needed
    #[inline(always)]
    fn addr(&mut self, ext: bool) -> RegT {
        if ext {
            let addr = (self.reg.r16sp(2) + self.d()) & 0xFFFF;
            self.reg.set_wz(addr);
            addr
        }
        else {
            self.reg.r16sp(2)
        }
    }

    /// check condition (for conditional jumps etc)
    #[inline(always)]
    fn cc(&self, y: usize) -> bool {
        let f = self.reg.f();
        match y {
            0 => 0 == f & ZF, // JR NZ
            1 => 0 != f & ZF, // JR Z
            2 => 0 == f & CF, // JR NC
            3 => 0 != f & CF, // JC C
            4 => 0 == f & PF, // JR PO
            5 => 0 != f & PF, // JR PE
            6 => 0 == f & SF, // JR P
            7 => 0 != f & SF, // JR M
            _ => false,
        }
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
    pub fn do_op(&mut self, ext: bool) -> i64 {
        let (cyc, ext_cyc) = if ext {(4,8)} else {(0,0)};
        let op = self.fetch_op();

        // split instruction byte into bit groups
        let x = op>>6;
        let y = (op>>3 & 7) as usize;
        let z = (op & 7) as usize;
        let p = y>>1;
        let q = y & 1;
        cyc + match (x, y, z) {
        //--- block 1: 8-bit loads
            // special case LD (HL),(HL): HALT
            (1, 6, 6) => {
                self.halt(); 
                4
            },
            // LD (HL),r; LD (IX+d),r; LD (IY+d),r
            // NOTE: this always loads from H,L, never IXH, ...
            (1, 6, _) => {
                let a = self.addr(ext);
                let v = self.reg.r8i(z);
                self.mem.w8(a, v);
                7 + ext_cyc
            },
            // LD r,(HL); LD r,(IX+d); LD r,(IY+d)
            // NOTE: this always loads to H,L, never IXH,...
            (1, _, 6) => {
                let a = self.addr(ext);
                let v = self.mem.r8(a);
                self.reg.set_r8i(y, v);
                7 + ext_cyc 
            },
            // LD r,s
            (1, _, _) => {
                let v = self.reg.r8(z);
                self.reg.set_r8(y, v);
                4
            },
        //--- block 2: 8-bit ALU instructions
            // ALU (HL); ALU (IX+d); ALU (IY+d)
            (2, _, _) => {
                if z == 6 {
                    // ALU (HL); ALU (IX+d); ALU (IY+d)
                    let a = self.addr(ext);
                    let val = self.mem.r8(a);
                    self.alu8(y, val);
                    7 + ext_cyc
                }
                else {
                    // ALU r
                    let val = self.reg.r8(z);
                    self.alu8(y, val);
                    4
                }
            },
        //--- block 0: misc ops
            // NOP
            (0, 0, 0) => { 
                4 
            },
            // EX AF,AF'
            (0, 1, 0) => { 
                self.reg.swap(AF, AF_); 
                4 
            },
            // DJNZ
            (0, 2, 0) => { 
                self.djnz() 
            },
            // JR d
            (0, 3, 0) => {
                let pc = self.reg.pc();
                let wz = pc + self.mem.rs8(pc) + 1;
                self.reg.set_pc(wz);
                self.reg.set_wz(wz);
                12
            },
            // JR cc
            (0, _, 0) => {
                let pc = self.reg.pc();
                if self.cc(y-4) {
                    let wz = pc + self.mem.rs8(pc) + 1;
                    self.reg.set_pc(wz);
                    self.reg.set_wz(wz);
                    12
                }
                else {
                    self.reg.set_pc(pc + 1);
                    7                    
                }
            }
            // 16-bit immediate loads and 16-bit ADD
            (0, _, 1) => {
                if q == 0 {
                    // LD rr,nn (inkl IX,IY)
                    let val = self.imm16();
                    self.reg.set_r16sp(p, val);
                    10
                }
                else {
                    // ADD HL,rr; ADD IX,rr; ADD IY,rr
                    let acc = self.reg.r16sp(2);
                    let val = self.reg.r16sp(p);
                    let res = self.add16(acc, val);
                    self.reg.set_r16sp(2, res);
                    11
                }
            },
            (0, _, 2) => {
                // indirect loads
                match (q, p) {
                    // LD (nn),HL; LD (nn),IX; LD (nn),IY
                    (0, 2) => {
                        let addr = self.imm16();
                        let v = self.reg.r16sp(2);
                        self.mem.w16(addr, v);
                        self.reg.set_wz(addr + 1);
                        16
                    },
                    // LD (nn),A
                    (0, 3) => {
                        let addr = self.imm16();
                        let a = self.reg.a();
                        self.mem.w8(addr, a);
                        self.reg.set_wz(addr + 1);
                        13
                    }
                    // LD (BC),A; LD (DE),A,; LD (nn),A
                    (0, _) => {
                        let addr = if p==0 { self.reg.bc() } else { self.reg.de() };
                        let a = self.reg.a();
                        self.mem.w8(addr, a);
                        self.reg.set_wz(a<<8 | ((addr+1) & 0xFF));
                        7
                    },
                    // LD HL,(nn); LD IX,(nn); LD IY,(nn)
                    (1, 2) => {
                        let addr = self.imm16();
                        let val  = self.mem.r16(addr);
                        self.reg.set_r16sp(2, val);
                        self.reg.set_wz(addr + 1);
                        16
                    },
                    // LD A,(nn)
                    (1, 3) => {
                        let addr = self.imm16();
                        let val = self.mem.r8(addr);
                        self.reg.set_a(val);
                        self.reg.set_wz(addr + 1);
                        13
                    }
                    // LD A,(BC); LD A,(DE)
                    (1, _) => {
                        let addr = if p == 0 { self.reg.bc() } else { self.reg.de() };
                        let val = self.mem.r8(addr);
                        self.reg.set_a(val);
                        self.reg.set_wz(addr + 1);
                        7
                    },
                    (_,_) => canthappen()
                }
            },
            (0, _, 3) => {
                // 16-bit INC/DEC
                let val = self.reg.r16sp(p) + if q==0 {1} else {-1};
                self.reg.set_r16sp(p, val);
                6
            },
            // INC (HL); INC (IX+d); INC (IY+d)
            (0, 6, 4) => {
                let addr = self.addr(ext);
                let v = self.mem.r8(addr);
                let w = self.inc8(v);
                self.mem.w8(addr, w);
                11 + ext_cyc
            },
            // INC r
            (0, _, 4) => {
                let v = self.reg.r8(y);
                let w = self.inc8(v);
                self.reg.set_r8(y, w);
                4
            },
            // DEC (HL); DEC (IX+d); DEC (IY+d)
            (0, 6, 5) => {
                let addr = self.addr(ext);
                let v = self.mem.r8(addr);
                let w = self.dec8(v);
                self.mem.w8(addr, w);
                11 + ext_cyc
            },
            // DEC r
            (0, _, 5) => {
                let v = self.reg.r8(y);
                let w = self.dec8(v);
                self.reg.set_r8(y, w);
                4
            },
            // LD r,n; LD (HL),n; LD (IX+d),n; LD (IY+d),n
            (0, _, 6) => {
                if y == 6 {
                    // LD (HL),n; LD (IX+d),n; LD (IY+d),n
                    let addr = self.addr(ext);
                    let v = self.imm8();
                    self.mem.w8(addr, v);
                    if ext { 15 } else { 10 }
                }
                else {
                    // LD r,n
                    let v = self.imm8();
                    self.reg.set_r8(y, v);
                    7
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
                    _ => canthappen()
                }
                4
            }
        //--- block 3: misc and prefixed ops
            (3, _, 0) => { 
                // RET cc
                self.retcc(y) 
            }
            (3, _, 1) => {
                match (q,p) {
                    (0, _) => {
                        // POP BC,DE,HL,IX,IY
                        let val = self.pop();
                        self.reg.set_r16af(p, val);
                        10
                    },
                    (1, 0) => {
                        // RET
                        self.ret()
                    },
                    (1, 1) => {
                        // EXX
                        self.reg.swap(BC, BC_);
                        self.reg.swap(DE, DE_);
                        self.reg.swap(HL, HL_);
                        self.reg.swap(WZ, WZ_);
                        4
                    },
                    (1, 2) => {
                        // JP HL; JP IX; JP IY
                        let v = self.reg.r16sp(2);
                        self.reg.set_pc(v);
                        4
                    },
                    (1, 3) => {
                        // LD SP,HL, LD SP,IX; LD SP,IY
                        let v = self.reg.r16sp(2);
                        self.reg.set_sp(v);
                        6
                    },
                    (_, _) => {
                        canthappen()
                    }
                }
            },
            (3, _, 2) => {
                // JP cc,nn
                let nn = self.imm16();
                self.reg.set_wz(nn);
                if self.cc(y) {
                    self.reg.set_pc(nn);
                }
                10
            },
            (3, _, 3) => {
                // misc ops
                match y {
                    0 => { 
                        // JP nn
                        let nn = self.imm16();
                        self.reg.set_wz(nn);
                        self.reg.set_pc(nn);
                        10
                    },
                    1 => {
                        self.do_cb_op(ext)
                    },
                    2 => {
                        // OUT (n),A
                        let a = self.reg.a();
                        let port = (a<<8 | self.imm8()) & 0xFFFF;
                        self.outp(port, a);
                        11
                    },
                    3 => {
                        // IN A,(n)
                        let port = (self.reg.a()<<8 | self.imm8()) & 0xFFFF;
                        let v = self.inp(port);
                        self.reg.set_a(v);
                        11
                    },
                    4 => {
                        // EX (SP),HL; EX (SP),IX; EX (SP),IY
                        let sp = self.reg.sp();
                        let v_reg = self.reg.r16sp(2);
                        let v_mem = self.mem.r16(sp);
                        self.mem.w16(sp, v_reg);
                        self.reg.set_wz(v_mem);
                        self.reg.set_r16sp(2, v_mem);
                        19
                    },
                    5 => {
                        // EX DE,HL
                        self.reg.swap(DE, HL);
                        4
                    },
                    6 => {
                        // DI
                        self.iff1 = false; 
                        self.iff2 = false;
                        4
                    },
                    7 => {
                        // EI
                        self.enable_interrupt = true;
                        4
                    },
                    _ => canthappen()
                }
            },
            (3, _, 4) => {
                // CALL cc
                self.callcc(y)
            },
            (3, _, 5) => {
                match (q, p) {
                    (0, _) => {
                        // PUSH BC,DE,HL,IX,IY,AF
                        let v = self.reg.r16af(p);
                        self.push(v);
                        11
                    },
                    (1, 0) => {
                        // CALL nn
                        self.call()
                    },
                    (1, 1) => {
                        // DD prefix instructions
                        self.reg.patch_ix();
                        let cycles = self.do_op(true);
                        self.reg.unpatch();
                        cycles
                    },
                    (1, 2) => {
                        // ED prefix instructions
                        self.do_ed_op()
                    },
                    (1, 3) => {
                        // FD prefix instructions
                        self.reg.patch_iy();
                        let cycles = self.do_op(true); 
                        self.reg.unpatch();
                        cycles
                    },
                    (_, _) => canthappen()
                }
            },
            // ALU n
            (3, _, 6) => {
                let val = self.imm8();
                self.alu8(y, val);
                7
            },
            // RST
            (3, _, 7) => {
                self.rst((y * 8) as RegT);
                11
            },
            // not implemented
            _ => {
                panic!("Invalid instruction!");
            }
        }
    }

    /// fetch and execute ED prefix instruction
    fn do_ed_op(&mut self) -> i64 {
        let op = self.fetch_op();

        // split instruction byte into bit groups
        let x = op>>6;
        let y = (op>>3 & 7) as usize;
        let z = (op & 7) as usize;
        let p = y>>1;
        let q = y & 1;
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

            (1, 6, 0) => { 
                // IN F,(C) (undocumented special case, only alter flags,
                // don't store result)
                let bc = self.reg.bc();
                let v = self.inp(bc);
                let f = flags_szp(v) | (self.reg.f() & CF);
                self.reg.set_f(f);
                12
            },
            (1, _, 0) => {
                // IN r,(C)
                let bc = self.reg.bc();
                let v = self.inp(bc);
                self.reg.set_r8(y, v);
                let f = flags_szp(v) | (self.reg.f() & CF);
                self.reg.set_f(f);
                12
            },
            (1, 6, 1) => {
                // OUT (C),F (undocumented special case, always output 0)
                let bc = self.reg.bc();
                self.outp(bc, 0);
                12
            },
            (1, _, 1) => {
                // OUT (C),r
                let bc = self.reg.bc();
                let v = self.reg.r8(y);
                self.outp(bc, v);
                12
            },
            (1, _, 2) => {
                // SBC/ADC HL,rr
                let acc = self.reg.hl();
                let val = self.reg.r16sp(p);
                let res = if q == 0 {
                    self.sbc16(acc, val)
                } 
                else {
                    self.adc16(acc, val)
                };
                self.reg.set_hl(res);
                15
            },
            (1, _, 3) => {
                // 16-bit immediate address load/store
                let nn = self.imm16();
                if q == 0 {
                    // LD (nn),rr
                    let val = self.reg.r16sp(p);
                    self.mem.w16(nn, val);
                }
                else {
                    // LD rr,(nn)
                    let val = self.mem.r16(nn);
                    self.reg.set_r16sp(p, val);
                }
                self.reg.set_wz(nn + 1);
                20
            },
            (1, _, 4) => {
                self.neg8();
                8
            },
            (1, _, 5) => {
                panic!("FIXME: RETN, RETI");
            },
            (1, _, 6) => {
                match y {
                    0|1|4|5 => { self.reg.im = 0; },
                    2|6     => { self.reg.im = 1; },
                    3|7     => { self.reg.im = 2; },
                    _       => { canthappen(); }
                }
                8
            },
            (1, 0, 7) => { self.reg.i = self.reg.a(); 9 },   // LD I,A
            (1, 1, 7) => { self.reg.r = self.reg.a(); 9 },   // LD R,A
            (1, 2, 7) => { 
                // LD A,I
                let i = self.reg.i;
                self.reg.set_a(i); 
                let f = flags_sziff2(i, self.iff2)|(self.reg.f() & CF);
                self.reg.set_f(f);
                9
            },
            (1, 3, 7) => {
                // LD A,R
                let r = self.reg.r;
                self.reg.set_a(r);
                let f = flags_sziff2(r, self.iff2)|(self.reg.f() & CF);
                self.reg.set_f(f);
                9
            },
            (1, 4, 7) => { self.rrd(); 18 },    // RRD
            (1, 5, 7) => { self.rld(); 18 },    // RLD
            (1, _, 7) => { 9 },     // NOP (ED)
            _ => panic!("CB: Invalid instruction!")
        }
    }

    /// fetch and execute CB prefix instruction
    fn do_cb_op(&mut self, ext: bool) -> i64 {
        let d = if ext {self.d()} else {0};
        let op = self.fetch_op();
        let cyc = if ext {4} else {0};

        // split instruction byte into bit groups
        let x = op>>6;
        let y = (op>>3 & 7) as usize;
        let z = (op & 7) as usize;
        cyc + match x {
            0 => {
                // rotates and shifts
                if z == 6 {
                    // ROT (HL); ROT (IX+d); ROT (IY+d)
                    let a = self.addr_d(d, ext);
                    let v = self.mem.r8(a);
                    let w = self.rot(y, v);
                    self.mem.w8(a, w);
                    15
                }
                else if ext {
                    // undocumented: ROT (IX+d), (IY+d),r 
                    // (also stores result in a register)
                    let a = self.addr_d(d, ext);
                    let v = self.mem.r8(a);
                    let w = self.rot(y, v);
                    self.reg.set_r8i(z, w);
                    self.mem.w8(a, w);
                    15
                }
                else {   
                    // ROT r
                    let v = self.reg.r8i(z);
                    let w = self.rot(y, v);
                    self.reg.set_r8i(z, w);
                    8
                }
            },
            1 => {
                // BIT n
                if z == 6 {
                    // BIT n,(HL); BIT n,(IX+d); BIT n,(IY+d)
                    let a = self.addr_d(d, ext);
                    let v = self.mem.r8(a);
                    self.ibit(v, 1<<y);
                    12
                }
                else {
                    // BIT n,r
                    let v = self.reg.r8i(z);
                    self.bit(v, 1<<y);
                    8
                }
            },
            2 => {
                // RES n
                if z == 6 {
                    // RES n,(HL); RES n,(IX+d); RES n,(IY+d)
                    let a = self.addr_d(d, ext);
                    let v = self.mem.r8(a) & !(1<<y);
                    self.mem.w8(a, v);
                    15
                }
                else if ext {
                    // RES n,(IX+d),r; RES n,(IY+d),r
                    // (also stores result in a register)
                    let a = self.addr_d(d, ext);
                    let v = self.mem.r8(a) & !(1<<y);
                    self.reg.set_r8i(z, v);
                    self.mem.w8(a, v);
                    15
                }
                else {
                    // RES n,r
                    let v = self.reg.r8i(z) & !(1<<y);
                    self.reg.set_r8i(z, v);
                    8
                }
            },
            3 => {
                // SET n
                if z == 6 {
                    // SET n,(HL); SET n,(IX+d); SET n,(IY+d)
                    let a = self.addr_d(d, ext);
                    let v = self.mem.r8(a) | 1<<y;
                    self.mem.w8(a, v);
                    15
                }
                else if ext {
                    // SET n,(IX+d),r; SET n,(IY+d),r
                    // (also stores result in a register)
                    let a = self.addr_d(d, ext);
                    let v = self.mem.r8(a) | 1<<y;
                    self.reg.set_r8i(z, v);
                    self.mem.w8(a, v);
                    15
                }
                else {
                    // SET n,r
                    let v = self.reg.r8i(z) | 1<<y;
                    self.reg.set_r8i(z, v);
                    8
                }
            },
            _ => canthappen() 
        }
    }

    pub fn halt(&mut self) {
        self.halt = true;
        let pc = self.reg.pc();
        self.reg.set_pc(pc - 1);
    }

    #[inline(always)]
    pub fn push(&mut self, val: RegT) {
        let addr = (self.reg.sp() - 2) & 0xFFFF;
        self.reg.set_sp(addr);
        self.mem.w16(addr, val);
    }

    #[inline(always)]
    pub fn pop(&mut self) -> RegT {
        let addr = self.reg.sp();
        let val = self.mem.r16(addr);
        self.reg.set_sp(addr + 2);
        val
    }

    #[inline(always)]
    pub fn rst(&mut self, val: RegT) {
        let pc = self.reg.pc();
        self.push(pc);
        self.reg.set_pc(val);
        self.reg.set_wz(val);
    }

    #[inline(always)]
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
            _ => panic!("Can't happen!")
        }
    }

    #[inline(always)]
    pub fn add8(&mut self, add: RegT) {
        let acc = self.reg.a();
        let res = acc + add;
        self.reg.set_f(flags_add(acc, add, res));
        self.reg.set_a(res);
    }

    #[inline(always)]
    pub fn adc8(&mut self, add: RegT) {
        let acc = self.reg.a();
        let res = acc + add + (self.reg.f() & CF);
        self.reg.set_f(flags_add(acc, add, res));
        self.reg.set_a(res);
    }

    #[inline(always)]
    pub fn sub8(&mut self, sub: RegT) {
        let acc = self.reg.a();
        let res = acc - sub;
        self.reg.set_f(flags_sub(acc, sub, res));
        self.reg.set_a(res);
    }

    #[inline(always)]
    pub fn sbc8(&mut self, sub: RegT) {
        let acc = self.reg.a();
        let res = acc - sub - (self.reg.f() & CF);
        self.reg.set_f(flags_sub(acc, sub, res));
        self.reg.set_a(res);
    }

    #[inline(always)]
    pub fn cp8(&mut self, sub: RegT) {
        let acc = self.reg.a();
        let res = acc - sub;
        self.reg.set_f(flags_cp(acc, sub, res));
    }

    #[inline(always)]
    pub fn neg8(&mut self) {
        let sub = self.reg.a();
        self.reg.set_a(0);
        self.sub8(sub);
    }

    #[inline(always)]
    pub fn and8(&mut self, val: RegT) {
        let res = self.reg.a() & val;
        self.reg.set_a(res);
        self.reg.set_f(flags_szp(res) | HF);
    }

    #[inline(always)]
    pub fn or8(&mut self, val: RegT) {
        let res = self.reg.a() | val;
        self.reg.set_a(res);
        self.reg.set_f(flags_szp(res));
    }

    #[inline(always)]
    pub fn xor8(&mut self, val: RegT) {
        let res = self.reg.a() ^ val;
        self.reg.set_a(res);
        self.reg.set_f(flags_szp(res));
    }

    #[inline(always)]
    pub fn inc8(&mut self, val: RegT) -> RegT {
        let res = (val + 1) & 0xFF;
        let f = (if res==0 {ZF} else {res & SF}) |
            (res & (XF|YF)) | 
            ((res^val) & HF) |
            (if res==0x80 {VF} else {0}) |
            (self.reg.f() & CF);
        self.reg.set_f(f);
        res
    }

    #[inline(always)]
    pub fn dec8(&mut self, val: RegT) -> RegT {
        let res = (val - 1) & 0xFF;
        let f = NF | 
            (if res==0 {ZF} else {res & SF}) |
            (res & (XF|YF)) |
            ((res^val) & HF) |
            (if res==0x7F {VF} else {0}) |
            (self.reg.f() & CF);
        self.reg.set_f(f);
        res
    }

    #[inline(always)]
    pub fn rot(&mut self, op: usize, val: RegT) -> RegT {
        match op {
            0 => self.rlc8(val),
            1 => self.rrc8(val),
            2 => self.rl8(val),
            3 => self.rr8(val),
            4 => self.sla8(val),
            5 => self.sra8(val),
            6 => self.sll8(val),
            7 => self.srl8(val),
            _ => panic!("Can't happen")
        }
    }

    #[inline(always)]
    pub fn rlc8(&mut self, val: RegT) -> RegT {
        let res = (val<<1 | val>>7) & 0xFF;
        self.reg.set_f(flags_szp(res) | ((val>>7) & CF));
        res
    }

    #[inline(always)]
    pub fn rlca8(&mut self) {
        let acc = self.reg.a();
        let res = (acc<<1 | acc>>7) & 0xFF;
        let f = ((acc>>7) & CF) | (res & (XF|YF)) | (self.reg.f() & (SF|ZF|PF));
        self.reg.set_f(f);
        self.reg.set_a(res);
    }

    #[inline(always)]
    pub fn rrc8(&mut self, val: RegT) -> RegT {
        let res = (val>>1 | val<<7) & 0xFF;
        self.reg.set_f(flags_szp(res) | (val & CF));
        res
    }

    #[inline(always)]
    pub fn rrca8(&mut self) {
        let acc = self.reg.a();
        let res = (acc>>1 | acc<<7) & 0xFF;
        let f = (acc & CF) | (res & (XF|YF)) | (self.reg.f() & (SF|ZF|PF));
        self.reg.set_f(f);
        self.reg.set_a(res);
    }

    #[inline(always)]
    pub fn rl8(&mut self, val: RegT) -> RegT {
        let res = (val<<1 | (self.reg.f() & CF)) & 0xFF;
        self.reg.set_f(flags_szp(res) | ((val>>7) & CF));
        res
    }

    #[inline(always)]
    pub fn rla8(&mut self) {
        let acc = self.reg.a();
        let f = self.reg.f();
        let res = (acc<<1 | (f & CF)) & 0xFF;
        self.reg.set_f(((acc>>7) & CF) | (res & (XF|YF)) | (f & (SF|ZF|PF)));
        self.reg.set_a(res);
    }

    #[inline(always)]
    pub fn rr8(&mut self, val: RegT) -> RegT {
        let res = (val>>1 | (self.reg.f() & CF)<<7) & 0xFF;
        self.reg.set_f(flags_szp(res) | (val & CF));
        res
    }

    #[inline(always)]
    pub fn rra8(&mut self) {
        let acc = self.reg.a();
        let f = self.reg.f();
        let res = (acc>>1 | (f & CF)<<7) & 0xFF;
        self.reg.set_f((acc & CF) | (res & (XF|YF)) | (f & (SF|ZF|PF)));
        self.reg.set_a(res);
    }

    #[inline(always)]
    pub fn sla8(&mut self, val: RegT) -> RegT {
        let res = (val<<1) & 0xFF;
        self.reg.set_f(flags_szp(res) | (val>>7 & CF));
        res
    }

    #[inline(always)]
    pub fn sll8(&mut self, val: RegT) -> RegT {
        // undocumented, sll8 is identical with sla8, but shifts a 1 into LSB
        let res = (val<<1 | 1) & 0xFF;
        self.reg.set_f(flags_szp(res) | (val>>7 & CF));
        res
    }

    #[inline(always)]
    pub fn sra8(&mut self, val: RegT) -> RegT {
        let res = (val>>1 | (val & 0x80)) & 0xFF;
        self.reg.set_f(flags_szp(res) | (val & CF));
        res
    }

    #[inline(always)]
    pub fn srl8(&mut self, val: RegT) -> RegT {
        let res = val>>1 & 0xFF;
        self.reg.set_f(flags_szp(res) | (val & CF));
        res
    }
    
    #[inline(always)]
    pub fn rld(&mut self) {
        let addr = self.reg.hl();
        let v = self.mem.r8(addr);
        let ah = self.reg.a() & 0xF0;
        let al = self.reg.a() & 0x0F;
        let a = ah | (v>>4 & 0x0F);
        self.reg.set_a(a);
        self.mem.w8(addr, (v<<4 | al) & 0xFF);
        self.reg.set_wz(addr + 1);
        let f = flags_szp(a) | (self.reg.f() & CF);
        self.reg.set_f(f);
    }

    #[inline(always)]
    pub fn rrd(&mut self) {
        let addr = self.reg.hl();
        let v = self.mem.r8(addr);
        let ah = self.reg.a() & 0xF0;
        let al = self.reg.a() & 0x0F;
        let a = ah | (v & 0x0F);
        self.reg.set_a(a);
        self.mem.w8(addr, (v>>4 | al<<4) & 0xFF);
        self.reg.set_wz(addr + 1);
        let f = flags_szp(a) | (self.reg.f() & CF);
        self.reg.set_f(f);
    }

    #[inline(always)]
    pub fn bit(&mut self, val: RegT, mask: RegT) {
        let res = val & mask;
        let f = HF | (self.reg.f() & CF) |
            (if res == 0 {ZF|PF} else {res & SF}) |
            (val & (XF|YF));
        self.reg.set_f(f)
    }

    #[inline(always)]
    pub fn ibit(&mut self, val: RegT, mask: RegT) {
        // special version of the BIT instruction for 
        // (HL), (IX+d), (IY+d) to set the undocumented XF|YF flags
        // from high byte of HL+1 or IX/IY+d (expected in WZ)
        let res = val & mask;
        let f = HF | (self.reg.f() & CF) |
            (if res == 0 {ZF|PF} else {res & SF}) |
            (self.reg.w() & (XF|YF));
        self.reg.set_f(f)
    }

    #[inline(always)]
    pub fn add16(&mut self, acc: RegT, add: RegT) -> RegT {
        self.reg.set_wz(acc + 1);
        let res = acc + add;
        let f = (self.reg.f() & (SF|ZF|VF)) |
            (((acc^res^add)>>8) & HF) |
            (res>>16 & CF) | (res>>8 & (YF|XF));
        self.reg.set_f(f);
        res & 0xFFFF
    }
    
    #[inline(always)]
    pub fn adc16(&mut self, acc: RegT, add: RegT) -> RegT {
        self.reg.set_wz(acc + 1);
        let res = acc + add + (self.reg.f() & CF);
        self.reg.set_f((((acc^res^add)>>8) & HF) |
            ((res>>16) & CF) |
            ((res>>8) & (SF|XF|YF)) |
            (if (res & 0xFFFF) == 0 {ZF} else {0}) |
            (((add^acc^0x8000) & (add^res) & 0x8000)>>13));
        res & 0xFFFF
    }

    #[inline(always)]
    pub fn sbc16(&mut self, acc: RegT, sub: RegT) -> RegT {
        self.reg.set_wz(acc + 1);
        let res = acc - sub - (self.reg.f() & CF);
        self.reg.set_f(NF | (((acc^res^sub)>>8) & HF) |
            ((res>>16) & CF) |
            ((res>>8) & (SF|XF|YF)) |
            (if (res & 0xFFFF) == 0 {ZF} else {0}) |
            (((sub^acc) & (acc^res) & 0x8000)>>13));
        res & 0xFFFF
    }

    #[inline(always)]
    pub fn djnz(&mut self) -> i64 {
        let b = (self.reg.b() - 1) & 0xFF;
        self.reg.set_b(b);
        if b > 0 {
            let addr = self.reg.pc();
            let d = self.mem.rs8(addr);
            let wz = addr + d + 1;
            self.reg.set_wz(wz);
            self.reg.set_pc(wz);
            13  // return num cycles if branch taken
        }
        else {
            let pc = self.reg.pc() + 1;
            self.reg.set_pc(pc);
            8   // return num cycles if loop finished
        }
    }

    #[inline(always)]
    pub fn daa(&mut self) {
        let a = self.reg.a();
        let mut val = a;
        let f = self.reg.f();
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
        self.reg.set_f((f & (CF|NF)) |
            (if a>0x99 {CF} else {0}) |
            ((a^val) & HF) |
            flags_szp(val));
        self.reg.set_a(val);
    }

    #[inline(always)]
    pub fn cpl(&mut self) {
        let f = self.reg.f();
        let a = self.reg.a() ^ 0xFF;
        self.reg.set_f((f & (SF|ZF|PF|CF)) | (HF|NF) | (a & (YF|XF)));
        self.reg.set_a(a);
    }

    #[inline(always)]
    pub fn scf(&mut self) {
        let f = self.reg.f();
        let a = self.reg.a();
        self.reg.set_f((f & (SF|ZF|YF|XF|PF)) | CF | (a & (YF|XF)));
    }

    #[inline(always)]
    pub fn ccf(&mut self) {
        let f = self.reg.f();
        let a = self.reg.a();
        self.reg.set_f(((f & (SF|ZF|YF|XF|PF|CF)) | ((f & CF)<<4) | (a & (YF|XF))) ^ CF);
    }

    #[inline(always)]
    pub fn ret(&mut self) -> i64 {
        let sp = self.reg.sp();
        let wz = self.mem.r16(sp);
        self.reg.set_wz(wz);
        self.reg.set_pc(wz);
        self.reg.set_sp(sp + 2);
        10
    }

    #[inline(always)]
    pub fn call(&mut self) -> i64 {
        let wz = self.imm16();
        let sp = (self.reg.sp() - 2) & 0xFFFF;
        self.mem.w16(sp, self.reg.pc());
        self.reg.set_sp(sp);
        self.reg.set_wz(wz);
        self.reg.set_pc(wz);
        17 
    }

    #[inline(always)]
    pub fn retcc(&mut self, y: usize) -> i64 {
        if self.cc(y) {
            self.ret() + 1
        }
        else {
            5
        }
    }

    #[inline(always)]
    pub fn callcc(&mut self, y: usize) -> i64 {
        if self.cc(y) {
            self.call()
        }
        else {
            let wz = self.imm16();
            self.reg.set_wz(wz);
            10
        }               
    }

    #[inline(always)]
    pub fn ldi(&mut self) {
        let hl = self.reg.hl();
        let de = self.reg.de();
        let val = self.mem.r8(hl);
        self.mem.w8(de, val);
        self.reg.set_hl(hl + 1);
        self.reg.set_de(de + 1);
        let bc = (self.reg.bc() - 1) & 0xFFFF;
        self.reg.set_bc(bc);
        let n = (val + self.reg.a()) & 0xFF;
        let f = (self.reg.f() & (SF|ZF|CF)) |
            (if (n & 0x02) != 0 {YF} else {0}) |
            (if (n & 0x08) != 0 {XF} else {0}) |
            (if bc > 0 {VF} else {0});
        self.reg.set_f(f);
    }

    #[inline(always)]
    pub fn ldd(&mut self) {
        let hl = self.reg.hl();
        let de = self.reg.de();
        let val = self.mem.r8(hl);
        self.mem.w8(de, val);
        self.reg.set_hl(hl - 1);
        self.reg.set_de(de - 1);
        let bc = (self.reg.bc() - 1) & 0xFFFF;
        self.reg.set_bc(bc);
        let n = (val + self.reg.a()) & 0xFF;
        let f = (self.reg.f() & (SF|ZF|CF)) |
            (if (n & 0x02) != 0 {YF} else {0}) |
            (if (n & 0x08) != 0 {XF} else {0}) |
            (if bc > 0 {VF} else {0});
        self.reg.set_f(f);
    }

    #[inline(always)]
    pub fn ldir(&mut self) -> i64 {
        self.ldi();
        if (self.reg.f() & VF) != 0 {
            let pc = self.reg.pc();
            self.reg.set_pc(pc - 2);
            self.reg.set_wz(pc + 1);
            21
        }
        else {
            16
        }
    }

    #[inline(always)]
    pub fn lddr(&mut self) -> i64 {
        self.ldd();
        if (self.reg.f() & VF) != 0 {
            let pc = self.reg.pc();
            self.reg.set_pc(pc - 2);
            self.reg.set_wz(pc + 1);
            21
        }
        else {
            16
        }
    }

    #[inline(always)]
    pub fn cpi(&mut self) {
        let wz = self.reg.wz();
        self.reg.set_wz(wz + 1);
        let hl = self.reg.hl();
        self.reg.set_hl(hl + 1);
        let bc = (self.reg.bc() - 1) & 0xFFFF;
        self.reg.set_bc(bc);
        let a = self.reg.a();
        let mut v = a - self.mem.r8(hl);
        let mut f = NF | (self.reg.f() & CF) |
            (if v==0 {ZF} else {v & SF}) |
            (if (v & 0xF)>(a & 0xF) {HF} else {0}) |
            (if bc != 0 {VF} else {0});
        if (f & HF) != 0 {
            v -= 1;
        }
        if (v & 0x02) != 0 { f |= YF };
        if (v & 0x08) != 0 { f |= XF };
        self.reg.set_f(f);
    }

    #[inline(always)]
    pub fn cpd(&mut self) {
        let wz = self.reg.wz();
        self.reg.set_wz(wz - 1);
        let hl = self.reg.hl();
        self.reg.set_hl(hl - 1);
        let bc = (self.reg.bc() - 1) & 0xFFFF;
        self.reg.set_bc(bc);
        let a = self.reg.a();
        let mut v = a - self.mem.r8(hl);
        let mut f = NF | (self.reg.f() & CF) |
            (if v==0 {ZF} else {v & SF}) |
            (if (v & 0xF)>(a & 0xF) {HF} else {0}) |
            (if bc != 0 {VF} else {0});
        if (f & HF) != 0 {
            v -= 1;
        }
        if (v & 0x02) != 0 { f |= YF };
        if (v & 0x08) != 0 { f |= XF };
        self.reg.set_f(f);
    }

    #[inline(always)]
    pub fn cpir(&mut self) -> i64 {
        self.cpi();
        if (self.reg.f() & (VF|ZF)) == VF {
            let pc = self.reg.pc();
            self.reg.set_pc(pc - 2);
            self.reg.set_wz(pc + 1);
            21
        }
        else {
            16
        }
    }

    #[inline(always)]
    pub fn cpdr(&mut self) -> i64 {
        self.cpd();
        if (self.reg.f() & (VF|ZF)) == VF {
            let pc = self.reg.pc();
            self.reg.set_pc(pc - 2);
            self.reg.set_wz(pc + 1);
            21
        }
        else {
            16
        }
    }

    #[inline(always)]
    pub fn inp(&mut self, port: RegT) -> RegT {
        (self.in_fn)(port) & 0xFF 
    }

    #[inline(always)]
    pub fn outp(&mut self, port: RegT, val: RegT) {
        (self.out_fn)(port, val)
    }

    #[inline(always)]
    fn ini_ind_flags(&self, val: RegT, add: RegT) -> RegT {
        let b = self.reg.b();
        let c = self.reg.c();
        let t = ((c + add) & 0xFF) + val;
        (if b != 0 {b & SF} else {ZF}) |
        (if (val & SF) != 0 {NF} else {0}) |
        (if (t & 0x100) != 0 {HF|CF} else {0}) |
        (flags_szp((t & 0x07) ^ b) & PF)
    }

    #[inline(always)]
    fn outi_outd_flags(&self, val: RegT) -> RegT {
        let b = self.reg.b();
        let l = self.reg.l();
        let t = l + val;
        (if b != 0 {b & SF} else {ZF}) |
        (if (val & SF) != 0 {NF} else {0}) |
        (if (t & 0x100) != 0 {HF|CF} else {0}) |
        (flags_szp((t & 0x07) ^ b) & PF)
    }

    #[inline(always)]
    pub fn ini(&mut self) {
        let bc = self.reg.bc();
        let io_val = self.inp(bc);
        self.reg.set_wz(bc + 1);
        let b = self.reg.b();
        self.reg.set_b(b - 1);
        let hl = self.reg.hl();
        self.mem.w8(hl, io_val);
        self.reg.set_hl(hl + 1);
        let f = self.ini_ind_flags(io_val, 1);
        self.reg.set_f(f);
    }

    #[inline(always)]
    pub fn ind(&mut self) {
        let bc = self.reg.bc();
        let io_val = self.inp(bc);
        self.reg.set_wz(bc - 1);
        let b = self.reg.b();
        self.reg.set_b(b - 1);
        let hl = self.reg.hl();
        self.mem.w8(hl, io_val);
        self.reg.set_hl(hl - 1);
        let f = self.ini_ind_flags(io_val, -1);
        self.reg.set_f(f);
    }

    #[inline(always)]
    pub fn inir(&mut self) -> i64 {
        self.ini();
        if self.reg.b() != 0 {
            let pc = self.reg.pc();
            self.reg.set_pc(pc - 2);
            21
        }
        else {
            16
        }
    }

    #[inline(always)]
    pub fn indr(&mut self) -> i64 {
        self.ind();
        if self.reg.b() != 0 {
            let pc = self.reg.pc();
            self.reg.set_pc(pc - 2);
            21
        }
        else {
            16
        }
    }

    #[inline(always)]
    pub fn outi(&mut self) {
        let hl = self.reg.hl();
        let io_val = self.mem.r8(hl);
        self.reg.set_hl(hl + 1);
        let b = self.reg.b();
        self.reg.set_b(b - 1);
        let bc = self.reg.bc();
        self.outp(bc, io_val);
        self.reg.set_wz(bc + 1);
        let f = self.outi_outd_flags(io_val);
        self.reg.set_f(f);
    }

    #[inline(always)]
    pub fn outd(&mut self) {
        let hl = self.reg.hl();
        let io_val = self.mem.r8(hl);
        self.reg.set_hl(hl - 1);
        let b = self.reg.b();
        self.reg.set_b(b - 1);
        let bc = self.reg.bc();
        self.outp(bc, io_val);
        self.reg.set_wz(bc - 1);
        let f = self.outi_outd_flags(io_val);
        self.reg.set_f(f);
    }

    #[inline(always)]
    pub fn otir(&mut self) -> i64 {
        self.outi();
        if self.reg.b() != 0 {
            let pc = self.reg.pc();
            self.reg.set_pc(pc - 2);
            21
        }
        else {
            16
        }
    }

    #[inline(always)]
    pub fn otdr(&mut self) -> i64 {
        self.outd();
        if self.reg.b() != 0 {
            let pc = self.reg.pc();
            self.reg.set_pc(pc - 2);
            21
        }
        else {
            16
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use RegT;
    use registers::CF as CF;
    use registers::NF as NF;
    use registers::VF as VF;
    use registers::PF as PF;
    use registers::XF as XF;
    use registers::HF as HF;
    use registers::YF as YF;
    use registers::ZF as ZF;
    use registers::SF as SF;

    fn in_fn(_: RegT) -> RegT { 0 }
    fn out_fn(_: RegT, _: RegT) { }

    #[test]
    fn reset() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_pc(0x1234);
        cpu.reg.set_wz(1234);
        cpu.reg.im = 45;
        cpu.halt = true;
        cpu.iff1 = true;
        cpu.iff2 = true;
        cpu.reg.i = 2;
        cpu.reg.r = 3;
        cpu.reset();
        assert!(0 == cpu.reg.pc());
        assert!(0 == cpu.reg.wz());
        assert!(0 == cpu.reg.im);
        assert!(!cpu.halt);
        assert!(!cpu.iff1);
        assert!(!cpu.iff2);
        assert!(0 == cpu.reg.i);
        assert!(0 == cpu.reg.r);
    }

    #[test]
    fn reg16_rw() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_bc(0x1234);
        cpu.reg.set_de(0x5678);
        cpu.reg.set_hl(0x1357);
        cpu.reg.set_af(0x1122);
        assert!(0x12 == cpu.reg.b());
        assert!(0x34 == cpu.reg.c());
        assert!(0x1234 == cpu.reg.bc());
        assert!(0x56 == cpu.reg.d());
        assert!(0x78 == cpu.reg.e());
        assert!(0x5678 == cpu.reg.de());
        assert!(0x13 == cpu.reg.h());
        assert!(0x57 == cpu.reg.l());
        assert!(0x1357 == cpu.reg.hl());
        assert!(0x22 == cpu.reg.f());
        assert!(0x11 == cpu.reg.a());
    }

    #[test]
    fn halt() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_pc(0x1234);
        cpu.halt();
        assert!(cpu.halt);
        assert!(0x1233 == cpu.reg.pc());
    }

    #[test]
    fn rst() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_pc(0x123);
        cpu.reg.set_sp(0x100);
        cpu.rst(0x38);
        assert!(0xFE == cpu.reg.sp());
        assert!(cpu.mem.r16(cpu.reg.sp()) == 0x123);
        assert!(0x38 == cpu.reg.pc());
        assert!(0x38 == cpu.reg.wz());
    }

    #[test]
    fn push() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_sp(0x100);
        cpu.push(0x1234);
        assert!(0xFE == cpu.reg.sp());
        assert!(cpu.mem.r16(cpu.reg.sp()) == 0x1234);
    }

    fn test_flags<I:FnMut(RegT)->RegT,O:FnMut(RegT,RegT)>(cpu: &CPU<I,O>, expected: RegT) -> bool {
        (cpu.reg.f() & !(XF|YF)) == expected
    }

    #[test]
    fn add8() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_a(0xF);
        cpu.add8(0xF);
        assert!(0x1E == cpu.reg.a());
        assert!(test_flags(&cpu, HF));
        cpu.add8(0xE0);
        assert!(0xFE == cpu.reg.a());
        assert!(test_flags(&cpu, SF));
        cpu.reg.set_a(0x81);
        cpu.add8(0x80);
        assert!(0x01 == cpu.reg.a());
        assert!(test_flags(&cpu, VF|CF));
        cpu.add8(0xFF);
        assert!(0x00 == cpu.reg.a());
        assert!(test_flags(&cpu, ZF|HF|CF));
    }

    #[test]
    fn adc8() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_a(0x00);
        cpu.adc8(0x00);
        assert!(0x00 == cpu.reg.a());
        assert!(test_flags(&cpu, ZF));
        cpu.adc8(0x41);
        assert!(0x41 == cpu.reg.a());
        assert!(test_flags(&cpu, 0));
        cpu.adc8(0x61);
        assert!(0xA2 == cpu.reg.a());
        assert!(test_flags(&cpu, SF|VF));
        cpu.adc8(0x81);
        assert!(0x23 == cpu.reg.a());
        assert!(test_flags(&cpu, VF|CF));
        cpu.adc8(0x41);
        assert!(0x65 == cpu.reg.a());
        assert!(test_flags(&cpu, 0));
    }

    #[test]
    fn sub8() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_a(0x04);
        cpu.sub8(0x04);
        assert!(0x00 == cpu.reg.a());
        assert!(test_flags(&cpu, ZF|NF));
        cpu.sub8(0x01);
        assert!(0xFF == cpu.reg.a());
        assert!(test_flags(&cpu, SF|HF|NF|CF));
        cpu.sub8(0xF8);
        assert!(0x07 == cpu.reg.a());
        assert!(test_flags(&cpu, NF));
        cpu.sub8(0x0F);
        assert!(0xF8 == cpu.reg.a());
        assert!(test_flags(&cpu, SF|HF|NF|CF));
    }

    #[test]
    fn sbc8() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_a(0x04);
        cpu.sbc8(0x04);
        assert!(0x00 == cpu.reg.a());
        assert!(test_flags(&cpu, ZF|NF));
        cpu.sbc8(0x01);
        assert!(0xFF == cpu.reg.a());
        assert!(test_flags(&cpu, SF|HF|NF|CF));
        cpu.sbc8(0xF8);
        assert!(0x06 == cpu.reg.a());
        assert!(test_flags(&cpu, NF));
    }

    #[test]
    fn cp8() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_a(0x04);
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
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_a(0x01);
        cpu.neg8();
        assert!(0xFF == cpu.reg.a());
        assert!(test_flags(&cpu, SF|HF|NF|CF));
        cpu.reg.set_a(0x00);
        cpu.neg8();
        assert!(0x00 == cpu.reg.a());
        assert!(test_flags(&cpu, NF|ZF));
        cpu.reg.set_a(0x80);
        cpu.neg8();
        assert!(0x80 == cpu.reg.a());
        assert!(test_flags(&cpu, SF|VF|NF|CF))
    }

    #[test]
    fn and8() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_a(0xFF); cpu.and8(0x01);
        assert!(0x01 == cpu.reg.a()); assert!(test_flags(&cpu, HF));
        cpu.reg.set_a(0xFF); cpu.and8(0xAA);
        assert!(0xAA == cpu.reg.a()); assert!(test_flags(&cpu, SF|HF|PF));
        cpu.reg.set_a(0xFF); cpu.and8(0x03);
        assert!(0x03 == cpu.reg.a()); assert!(test_flags(&cpu, HF|PF));
    }

    #[test]
    fn or8() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_a(0x00); 
        cpu.or8(0x00);
        assert!(0x00 == cpu.reg.a()); assert!(test_flags(&cpu, ZF|PF));
        cpu.or8(0x01);
        assert!(0x01 == cpu.reg.a()); assert!(test_flags(&cpu, 0));
        cpu.or8(0x02);
        assert!(0x03 == cpu.reg.a()); assert!(test_flags(&cpu, PF));
    }

    #[test]
    fn xor8() {
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_a(0x00);
        cpu.xor8(0x00);
        assert!(0x00 == cpu.reg.a()); assert!(test_flags(&cpu, ZF|PF));
        cpu.xor8(0x01);
        assert!(0x01 == cpu.reg.a()); assert!(test_flags(&cpu, 0));
        cpu.xor8(0x03);
        assert!(0x02 == cpu.reg.a()); assert!(test_flags(&cpu, 0));
    }

    #[test]
    fn inc8_dec8() {
        let mut cpu = CPU::new(in_fn, out_fn);
        let a = cpu.inc8(0x00);
        assert!(0x01 == a); assert!(test_flags(&cpu, 0));
        let b = cpu.dec8(a);
        assert!(0x00 == b); assert!(test_flags(&cpu, ZF|NF));
        let c = cpu.inc8(0xFF);
        assert!(0x00 == c); assert!(test_flags(&cpu, ZF|HF));
        let d = cpu.dec8(c);
        let f = cpu.reg.f() | CF;
        cpu.reg.set_f(f);   // set carry flag (should be preserved)
        assert!(0xFF == d); assert!(test_flags(&cpu, SF|HF|NF|CF));
        let e = cpu.inc8(0x0F);
        assert!(0x10 == e); assert!(test_flags(&cpu, HF|CF));
        let f = cpu.dec8(e);
        assert!(0x0F == f); assert!(test_flags(&cpu, HF|NF|CF));
    }

    #[test]
    fn rlc8_rrc8() {
        let mut cpu = CPU::new(in_fn, out_fn);
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
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_f(0xFF);
        cpu.reg.set_a(0xA0);
        cpu.rlca8();
        assert!(0x41 == cpu.reg.a()); assert!(test_flags(&cpu, SF|ZF|VF|CF));
        cpu.rlca8();
        assert!(0x82 == cpu.reg.a()); assert!(test_flags(&cpu, SF|ZF|VF));
        cpu.rrca8();
        assert!(0x41 == cpu.reg.a()); assert!(test_flags(&cpu, SF|ZF|VF));
        cpu.rrca8();
        assert!(0xA0 == cpu.reg.a()); assert!(test_flags(&cpu, SF|ZF|VF|CF));
    }

    #[test]
    fn rl8_rr8() {
        let mut cpu = CPU::new(in_fn, out_fn);
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
        let mut cpu = CPU::new(in_fn, out_fn);
        cpu.reg.set_f(0xFF);
        cpu.reg.set_a(0xA0);
        cpu.rla8();
        assert!(0x41 == cpu.reg.a()); assert!(test_flags(&cpu, SF|ZF|VF|CF));
        cpu.rla8();
        assert!(0x83 == cpu.reg.a()); assert!(test_flags(&cpu, SF|ZF|VF));
        cpu.rra8();
        assert!(0x41 == cpu.reg.a()); assert!(test_flags(&cpu, SF|ZF|VF|CF));
        cpu.rra8();
        assert!(0xA0 == cpu.reg.a()); assert!(test_flags(&cpu, SF|ZF|VF|CF));
    }

    #[test]
    fn sla8() {
        let mut cpu = CPU::new(in_fn, out_fn);
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
        let mut cpu = CPU::new(in_fn, out_fn);
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
        let mut cpu = CPU::new(in_fn, out_fn);
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

    #[test]
    fn inp() {
        let in_func = |port| {
            assert!(port == 0x1234);
            port & 0xFF 
        };
        let mut cpu = CPU::new(in_func, out_fn);
        let i = cpu.inp(0x1234);
        assert!(i == 0x34);
    }

    #[test]
    fn outp() {
        let out_func = |port, val| {
            assert!(port == 0x1234);
            assert!(val == 12);
        };
        let mut cpu = CPU::new(in_fn, out_func);
        cpu.outp(0x1234, 12);
    }
}

