extern crate rz80;

#[cfg(test)]
#[allow(unused_imports)]
mod test_opcodes {
    use rz80;
    use rz80::A as A;
    use rz80::B as B;
    use rz80::C as C;
    use rz80::D as D;
    use rz80::E as E;
    use rz80::H as H;
    use rz80::L as L;
    use rz80::F as F;
    use rz80::BC as BC;
    use rz80::DE as DE;
    use rz80::HL as HL;
    use rz80::SP as SP;
    use rz80::CF as CF;
    use rz80::NF as NF;
    use rz80::VF as VF;
    use rz80::PF as PF;
    use rz80::XF as XF;
    use rz80::HF as HF;
    use rz80::YF as YF;
    use rz80::ZF as ZF;
    use rz80::SF as SF;

    fn flags(cpu: &rz80::CPU, expected: rz80::RegT) -> bool {
        (cpu.reg[F] & !(XF|YF)) == expected
    }
    
    #[test]
    fn test_ld_r_s() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x47,       // LD B,A
            0x4F,       // LD C,A
            0x57,       // LD D,A
            0x5F,       // LD E,A
            0x67,       // LD H,A
            0x6F,       // LD L,A
            0x7F,       // LD A,A

            0x48,       // LD C,B
            0x51,       // LD D,C
            0x5A,       // LD E,D
            0x63,       // LD H,E
            0x6C,       // LD L,H
            0x7D,       // LD A,L

        ];
        cpu.mem.write(0x0000, &prog);

        cpu.reg[A] = 0x12;
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[B]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[C]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[D]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[E]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[H]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[L]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[A]);
        cpu.reg[B] = 0x13;
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[C]);
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[D]);
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[E]);
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[H]);
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[L]);
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[A]);
    }

    #[test]
    fn test_ld_ihl() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x77,       // LD (HL),A
            0x46,       // LD B,(HL)
            0x4E,       // LD C,(HL)
            0x56,       // LD D,(HL)
            0x5E,       // LD E,(HL)
            0x66,       // LD H,(HL)
        ];
        cpu.mem.write(0x0100, &prog);

        cpu.reg[A] = 0x33;
        cpu.w16(HL, 0x1000);
        cpu.pc = 0x0100;
        assert!(7 == cpu.step()); assert!(0x33 == cpu.mem.r8(0x1000));
        assert!(7 == cpu.step()); assert!(0x33 == cpu.reg[B]);
        assert!(7 == cpu.step()); assert!(0x33 == cpu.reg[C]);
        assert!(7 == cpu.step()); assert!(0x33 == cpu.reg[D]);
        assert!(7 == cpu.step()); assert!(0x33 == cpu.reg[E]);
        assert!(7 == cpu.step()); assert!(0x33 == cpu.reg[H]);
    }

    #[test]
    fn test_add_r() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x3E, 0x0F,     // LD A,0x0F
            0x87,           // ADD A,A
            0x06, 0xE0,     // LD B,0xE0
            0x80,           // ADD A,B
            0x3E, 0x81,     // LD A,0x81
            0x0E, 0x80,     // LD C,0x80
            0x81,           // ADD A,C
            0x16, 0xFF,     // LD D,0xFF
            0x82,           // ADD A,D
            0x1E, 0x40,     // LD E,0x40
            0x83,           // ADD A,E
            0x26, 0x80,     // LD H,0x80
            0x84,           // ADD A,H
            0x2E, 0x33,     // LD L,0x33
            0x85,           // ADD A,L
            0xC6, 0x44,     // ADD A,0x44        
        ];
        cpu.mem.write(0x0000, &prog);

        assert!(7==cpu.step()); assert!(0x0F == cpu.reg[A]); assert!(flags(&cpu, 0));      
        assert!(4==cpu.step()); assert!(0x1E == cpu.reg[A]); assert!(flags(&cpu, HF));
        assert!(7==cpu.step()); assert!(0xE0 == cpu.reg[B]);                  
        assert!(4==cpu.step()); assert!(0xFE == cpu.reg[A]); assert!(flags(&cpu, SF));
        assert!(7==cpu.step()); assert!(0x81 == cpu.reg[A]);                  
        assert!(7==cpu.step()); assert!(0x80 == cpu.reg[C]);                  
        assert!(4==cpu.step()); assert!(0x01 == cpu.reg[A]); assert!(flags(&cpu, VF|CF));
        assert!(7==cpu.step()); assert!(0xFF == cpu.reg[D]);
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|HF|CF));
        assert!(7==cpu.step()); assert!(0x40 == cpu.reg[E]);                  
        assert!(4==cpu.step()); assert!(0x40 == cpu.reg[A]); assert!(flags(&cpu, 0));      
        assert!(7==cpu.step()); assert!(0x80 == cpu.reg[H]);                  
        assert!(4==cpu.step()); assert!(0xC0 == cpu.reg[A]); assert!(flags(&cpu, SF));
        assert!(7==cpu.step()); assert!(0x33 == cpu.reg[L]);                  
        assert!(4==cpu.step()); assert!(0xF3 == cpu.reg[A]); assert!(flags(&cpu, SF));
        assert!(7==cpu.step()); assert!(0x37 == cpu.reg[A]); assert!(flags(&cpu, CF));
    }

    #[test]
    fn test_adc_r() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x3E, 0x00,         // LD A,0x00
            0x06, 0x41,         // LD B,0x41
            0x0E, 0x61,         // LD C,0x61
            0x16, 0x81,         // LD D,0x81
            0x1E, 0x41,         // LD E,0x41
            0x26, 0x61,         // LD H,0x61
            0x2E, 0x81,         // LD L,0x81
            0x8F,               // ADC A,A
            0x88,               // ADC A,B
            0x89,               // ADC A,C
            0x8A,               // ADC A,D
            0x8B,               // ADC A,E
            0x8C,               // ADC A,H
            0x8D,               // ADC A,L
            0xCE, 0x01,         // ADC A,0x01
        ];
        cpu.mem.write(0x0000, &prog);

        assert!(7==cpu.step()); assert!(0x00 == cpu.reg[A]);
        assert!(7==cpu.step()); assert!(0x41 == cpu.reg[B]);
        assert!(7==cpu.step()); assert!(0x61 == cpu.reg[C]);
        assert!(7==cpu.step()); assert!(0x81 == cpu.reg[D]);
        assert!(7==cpu.step()); assert!(0x41 == cpu.reg[E]);
        assert!(7==cpu.step()); assert!(0x61 == cpu.reg[H]);
        assert!(7==cpu.step()); assert!(0x81 == cpu.reg[L]);
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF));
        assert!(4==cpu.step()); assert!(0x41 == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(4==cpu.step()); assert!(0xA2 == cpu.reg[A]); assert!(flags(&cpu, SF|VF));
        assert!(4==cpu.step()); assert!(0x23 == cpu.reg[A]); assert!(flags(&cpu, VF|CF));
        assert!(4==cpu.step()); assert!(0x65 == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(4==cpu.step()); assert!(0xC6 == cpu.reg[A]); assert!(flags(&cpu, SF|VF));
        assert!(4==cpu.step()); assert!(0x47 == cpu.reg[A]); assert!(flags(&cpu, VF|CF));
        assert!(7==cpu.step()); assert!(0x49 == cpu.reg[A]); assert!(flags(&cpu, 0));
    }

    #[test]
    fn test_sub_r() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x3E, 0x04,     // LD A,0x04
            0x06, 0x01,     // LD B,0x01
            0x0E, 0xF8,     // LD C,0xF8
            0x16, 0x0F,     // LD D,0x0F
            0x1E, 0x79,     // LD E,0x79
            0x26, 0xC0,     // LD H,0xC0
            0x2E, 0xBF,     // LD L,0xBF
            0x97,           // SUB A,A
            0x90,           // SUB A,B
            0x91,           // SUB A,C
            0x92,           // SUB A,D
            0x93,           // SUB A,E
            0x94,           // SUB A,H
            0x95,           // SUB A,L
            0xD6, 0x01,     // SUB A,0x01
            0xD6, 0xFE,     // SUB A,0xFE
        ];
        cpu.mem.write(0x0000, &prog);

        assert!(7==cpu.step()); assert!(0x04 == cpu.reg[A]);
        assert!(7==cpu.step()); assert!(0x01 == cpu.reg[B]);
        assert!(7==cpu.step()); assert!(0xF8 == cpu.reg[C]);
        assert!(7==cpu.step()); assert!(0x0F == cpu.reg[D]);
        assert!(7==cpu.step()); assert!(0x79 == cpu.reg[E]);
        assert!(7==cpu.step()); assert!(0xC0 == cpu.reg[H]);
        assert!(7==cpu.step()); assert!(0xBF == cpu.reg[L]);
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|NF));
        assert!(4==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF|CF));
        assert!(4==cpu.step()); assert!(0x07 == cpu.reg[A]); assert!(flags(&cpu, NF));
        assert!(4==cpu.step()); assert!(0xF8 == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF|CF));
        assert!(4==cpu.step()); assert!(0x7F == cpu.reg[A]); assert!(flags(&cpu, HF|VF|NF));
        assert!(4==cpu.step()); assert!(0xBF == cpu.reg[A]); assert!(flags(&cpu, SF|VF|NF|CF));
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|NF));
        assert!(7==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF|CF));
        assert!(7==cpu.step()); assert!(0x01 == cpu.reg[A]); assert!(flags(&cpu, NF));        
    }

    #[test]
    fn test_cp_r() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x3E, 0x04,     // LD A,0x04
            0x06, 0x05,     // LD B,0x05
            0x0E, 0x03,     // LD C,0x03
            0x16, 0xff,     // LD D,0xff
            0x1E, 0xaa,     // LD E,0xaa
            0x26, 0x80,     // LD H,0x80
            0x2E, 0x7f,     // LD L,0x7f
            0xBF,           // CP A
            0xB8,           // CP B
            0xB9,           // CP C
            0xBA,           // CP D
            0xBB,           // CP E
            0xBC,           // CP H
            0xBD,           // CP L
            0xFE, 0x04,     // CP 0x04
        ];
        cpu.mem.write(0x0000, &prog);

        assert!(7==cpu.step()); assert!(0x04 == cpu.reg[A]);
        assert!(7==cpu.step()); assert!(0x05 == cpu.reg[B]);
        assert!(7==cpu.step()); assert!(0x03 == cpu.reg[C]);
        assert!(7==cpu.step()); assert!(0xff == cpu.reg[D]);
        assert!(7==cpu.step()); assert!(0xaa == cpu.reg[E]);
        assert!(7==cpu.step()); assert!(0x80 == cpu.reg[H]);
        assert!(7==cpu.step()); assert!(0x7f == cpu.reg[L]);
        assert!(4==cpu.step()); assert!(0x04 == cpu.reg[A]); assert!(flags(&cpu, ZF|NF));
        assert!(4==cpu.step()); assert!(0x04 == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF|CF)); 
        assert!(4==cpu.step()); assert!(0x04 == cpu.reg[A]); assert!(flags(&cpu, NF));
        assert!(4==cpu.step()); assert!(0x04 == cpu.reg[A]); assert!(flags(&cpu, HF|NF|CF));
        assert!(4==cpu.step()); assert!(0x04 == cpu.reg[A]); assert!(flags(&cpu, HF|NF|CF));
        assert!(4==cpu.step()); assert!(0x04 == cpu.reg[A]); assert!(flags(&cpu, SF|VF|NF|CF));
        assert!(4==cpu.step()); assert!(0x04 == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF|CF));
        assert!(7==cpu.step()); assert!(0x04 == cpu.reg[A]); assert!(flags(&cpu, ZF|NF));        
    }

    #[test]
    fn test_sbc_r() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x3E, 0x04,     // LD A,0x04
            0x06, 0x01,     // LD B,0x01
            0x0E, 0xF8,     // LD C,0xF8
            0x16, 0x0F,     // LD D,0x0F
            0x1E, 0x79,     // LD E,0x79
            0x26, 0xC0,     // LD H,0xC0
            0x2E, 0xBF,     // LD L,0xBF
            0x97,           // SUB A,A
            0x98,           // SBC A,B
            0x99,           // SBC A,C
            0x9A,           // SBC A,D
            0x9B,           // SBC A,E
            0x9C,           // SBC A,H
            0x9D,           // SBC A,L
            0xDE, 0x01,     // SBC A,0x01
            0xDE, 0xFE,     // SBC A,0xFE
        ];
        cpu.mem.write(0x0000, &prog);

        for _ in 0..7 {
            cpu.step();
        }
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|NF));
        assert!(4==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF|CF));
        assert!(4==cpu.step()); assert!(0x06 == cpu.reg[A]); assert!(flags(&cpu, NF));
        assert!(4==cpu.step()); assert!(0xF7 == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF|CF));
        assert!(4==cpu.step()); assert!(0x7D == cpu.reg[A]); assert!(flags(&cpu, HF|VF|NF));
        assert!(4==cpu.step()); assert!(0xBD == cpu.reg[A]); assert!(flags(&cpu, SF|VF|NF|CF));
        assert!(4==cpu.step()); assert!(0xFD == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF|CF));
        assert!(7==cpu.step()); assert!(0xFB == cpu.reg[A]); assert!(flags(&cpu, SF|NF));
        assert!(7==cpu.step()); assert!(0xFD == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF|CF));        
    }

    #[test]
    fn test_or_r() {
        let mut cpu = rz80::CPU::new();

        let prog = [
            0x97,           // SUB A
            0x06, 0x01,     // LD B,0x01
            0x0E, 0x02,     // LD C,0x02
            0x16, 0x04,     // LD D,0x04
            0x1E, 0x08,     // LD E,0x08
            0x26, 0x10,     // LD H,0x10
            0x2E, 0x20,     // LD L,0x20
            0xB7,           // OR A
            0xB0,           // OR B
            0xB1,           // OR C
            0xB2,           // OR D
            0xB3,           // OR E
            0xB4,           // OR H
            0xB5,           // OR L
            0xF6, 0x40,     // OR 0x40
            0xF6, 0x80,     // OR 0x80
        ];
        cpu.mem.write(0x0000, &prog);

        for _ in 0..7 {
            cpu.step();
        }
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|PF));
        assert!(4==cpu.step()); assert!(0x01 == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(4==cpu.step()); assert!(0x03 == cpu.reg[A]); assert!(flags(&cpu, PF));
        assert!(4==cpu.step()); assert!(0x07 == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(4==cpu.step()); assert!(0x0F == cpu.reg[A]); assert!(flags(&cpu, PF));
        assert!(4==cpu.step()); assert!(0x1F == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(4==cpu.step()); assert!(0x3F == cpu.reg[A]); assert!(flags(&cpu, PF));
        assert!(7==cpu.step()); assert!(0x7F == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(7==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, SF|PF));        
    }
   
    #[test]
    fn text_xor_r() {
        let mut cpu = rz80::CPU::new();

        let prog = [
            0x97,           // SUB A
            0x06, 0x01,     // LD B,0x01
            0x0E, 0x03,     // LD C,0x03
            0x16, 0x07,     // LD D,0x07
            0x1E, 0x0F,     // LD E,0x0F
            0x26, 0x1F,     // LD H,0x1F
            0x2E, 0x3F,     // LD L,0x3F
            0xAF,           // XOR A
            0xA8,           // XOR B
            0xA9,           // XOR C
            0xAA,           // XOR D
            0xAB,           // XOR E
            0xAC,           // XOR H
            0xAD,           // XOR L
            0xEE, 0x7F,     // XOR 0x7F
            0xEE, 0xFF,     // XOR 0xFF
        ];
        cpu.mem.write(0x0000, &prog);

        for _ in 0..7 {
            cpu.step();
        }
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|PF));
        assert!(4==cpu.step()); assert!(0x01 == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(4==cpu.step()); assert!(0x02 == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(4==cpu.step()); assert!(0x05 == cpu.reg[A]); assert!(flags(&cpu, PF));
        assert!(4==cpu.step()); assert!(0x0A == cpu.reg[A]); assert!(flags(&cpu, PF));
        assert!(4==cpu.step()); assert!(0x15 == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(4==cpu.step()); assert!(0x2A == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(7==cpu.step()); assert!(0x55 == cpu.reg[A]); assert!(flags(&cpu, PF));
        assert!(7==cpu.step()); assert!(0xAA == cpu.reg[A]); assert!(flags(&cpu, SF|PF));
    }

    #[test]
    fn test_and_r() {
        let mut cpu = rz80::CPU::new();

        let prog = [
            0x3E, 0xFF,             // LD A,0xFF
            0x06, 0x01,             // LD B,0x01
            0x0E, 0x03,             // LD C,0x02
            0x16, 0x04,             // LD D,0x04
            0x1E, 0x08,             // LD E,0x08
            0x26, 0x10,             // LD H,0x10
            0x2E, 0x20,             // LD L,0x20
            0xA0,                   // AND B
            0xF6, 0xFF,             // OR 0xFF
            0xA1,                   // AND C
            0xF6, 0xFF,             // OR 0xFF
            0xA2,                   // AND D
            0xF6, 0xFF,             // OR 0xFF
            0xA3,                   // AND E
            0xF6, 0xFF,             // OR 0xFF
            0xA4,                   // AND H
            0xF6, 0xFF,             // OR 0xFF
            0xA5,                   // AND L
            0xF6, 0xFF,             // OR 0xFF
            0xE6, 0x40,             // AND 0x40
            0xF6, 0xFF,             // OR 0xFF
            0xE6, 0xAA,             // AND 0xAA
        ];
        cpu.mem.write(0x0000, &prog);

        for _ in 0..7 {
            cpu.step();
        }
        assert!(4==cpu.step()); assert!(0x01 == cpu.reg[A]); assert!(flags(&cpu, HF));
        assert!(7==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, SF|PF));
        assert!(4==cpu.step()); assert!(0x03 == cpu.reg[A]); assert!(flags(&cpu, HF|PF));
        assert!(7==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, SF|PF));
        assert!(4==cpu.step()); assert!(0x04 == cpu.reg[A]); assert!(flags(&cpu, HF));
        assert!(7==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, SF|PF));
        assert!(4==cpu.step()); assert!(0x08 == cpu.reg[A]); assert!(flags(&cpu, HF));
        assert!(7==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, SF|PF));
        assert!(4==cpu.step()); assert!(0x10 == cpu.reg[A]); assert!(flags(&cpu, HF));
        assert!(7==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, SF|PF));
        assert!(4==cpu.step()); assert!(0x20 == cpu.reg[A]); assert!(flags(&cpu, HF));
        assert!(7==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, SF|PF));
        assert!(7==cpu.step()); assert!(0x40 == cpu.reg[A]); assert!(flags(&cpu, HF));
        assert!(7==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, SF|PF));
        assert!(7==cpu.step()); assert!(0xAA == cpu.reg[A]); assert!(flags(&cpu, SF|HF|PF));        
    }

    #[test]
    fn test_inc_dec_r() {
        let mut cpu = rz80::CPU::new();

        let prog = [
            0x3e, 0x00,         // LD A,0x00
            0x06, 0xFF,         // LD B,0xFF
            0x0e, 0x0F,         // LD C,0x0F
            0x16, 0x0E,         // LD D,0x0E
            0x1E, 0x7F,         // LD E,0x7F
            0x26, 0x3E,         // LD H,0x3E
            0x2E, 0x23,         // LD L,0x23
            0x3C,               // INC A
            0x3D,               // DEC A
            0x04,               // INC B
            0x05,               // DEC B
            0x0C,               // INC C
            0x0D,               // DEC C
            0x14,               // INC D
            0x15,               // DEC D
            0xFE, 0x01,         // CP 0x01  // set carry flag (should be preserved)
            0x1C,               // INC E
            0x1D,               // DEC E
            0x24,               // INC H
            0x25,               // DEC H
            0x2C,               // INC L
            0x2D,               // DEC L
        ];
        cpu.mem.write(0x0000, &prog);

        for _ in 0..7 {
            cpu.step();
        }
        assert!(4==cpu.step()); assert!(0x01 == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|NF));
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[B]); assert!(flags(&cpu, ZF|HF));
        assert!(4==cpu.step()); assert!(0xFF == cpu.reg[B]); assert!(flags(&cpu, SF|HF|NF));
        assert!(4==cpu.step()); assert!(0x10 == cpu.reg[C]); assert!(flags(&cpu, HF));
        assert!(4==cpu.step()); assert!(0x0F == cpu.reg[C]); assert!(flags(&cpu, HF|NF));
        assert!(4==cpu.step()); assert!(0x0F == cpu.reg[D]); assert!(flags(&cpu, 0));
        assert!(4==cpu.step()); assert!(0x0E == cpu.reg[D]); assert!(flags(&cpu, NF));
        assert!(7==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF|CF));
        assert!(4==cpu.step()); assert!(0x80 == cpu.reg[E]); assert!(flags(&cpu, SF|HF|VF|CF));
        assert!(4==cpu.step()); assert!(0x7F == cpu.reg[E]); assert!(flags(&cpu, HF|VF|NF|CF));
        assert!(4==cpu.step()); assert!(0x3F == cpu.reg[H]); assert!(flags(&cpu, CF));
        assert!(4==cpu.step()); assert!(0x3E == cpu.reg[H]); assert!(flags(&cpu, NF|CF));
        assert!(4==cpu.step()); assert!(0x24 == cpu.reg[L]); assert!(flags(&cpu, CF));
        assert!(4==cpu.step()); assert!(0x23 == cpu.reg[L]); assert!(flags(&cpu, NF|CF));        
    }

    #[test]
    fn test_djnz() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x06, 0x03,     // LD BC,0x03
            0x97,           // SUB A
            0x3C,           // loop: INC A
            0x10, 0xFD,     // DJNZ loop
            0x00,           // NOP
        ];
        cpu.mem.write(0x0204, &prog);
        cpu.pc = 0x0204;

        assert!(7  == cpu.step()); assert!(0x03 == cpu.reg[B]);
        assert!(4  == cpu.step()); assert!(0x00 == cpu.reg[A]);
        assert!(4  == cpu.step()); assert!(0x01 == cpu.reg[A]);
        assert!(13 == cpu.step()); assert!(0x02 == cpu.reg[B]); assert!(0x0207 == cpu.pc);
        assert!(4  == cpu.step()); assert!(0x02 == cpu.reg[A]);
        assert!(13 == cpu.step()); assert!(0x01 == cpu.reg[B]); assert!(0x0207 == cpu.pc);
        assert!(4  == cpu.step()); assert!(0x03 == cpu.reg[A]);
        assert!(8  == cpu.step()); assert!(0x00 == cpu.reg[B]); assert!(0x020A == cpu.pc);
    }  

    #[test]
    fn test_jr_cc() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x97,           //      SUB A
            0x20, 0x03,     //      JR NZ l0
            0x28, 0x01,     //      JR Z, l0
            0x00,           //      NOP
            0xC6, 0x01,     // l0:  ADD A,0x01
            0x28, 0x03,     //      JR Z, l1
            0x20, 0x01,     //      HR NZ, l1
            0x00,           //      NOP
            0xD6, 0x03,     // l1:  SUB 0x03
            0x30, 0x03,     //      JR NC, l2
            0x38, 0x01,     //      JR C, l2
            0x00,           //      NOP
            0x00,           //      NOP
        ];
        cpu.mem.write(0x204, &prog);
        cpu.pc = 0x0204;

        assert!(4  == cpu.step()); assert!(0x00 == cpu.reg[A]);
        assert!(7  == cpu.step()); assert!(0x0207 == cpu.pc);
        assert!(12 == cpu.step()); assert!(0x020A == cpu.pc);
        assert!(7  == cpu.step()); assert!(0x01 == cpu.reg[A]);
        assert!(7  == cpu.step()); assert!(0x020E == cpu.pc);
        assert!(12 == cpu.step()); assert!(0x0211 == cpu.pc);
        assert!(7  == cpu.step()); assert!(0xFE == cpu.reg[A]);
        assert!(7  == cpu.step()); assert!(0x0215 == cpu.pc);
        assert!(12 == cpu.step()); assert!(0x0218 == cpu.pc);
    }

    #[test]
    fn test_ihl_r() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x21, 0x00, 0x10,   // LD HL,0x1000
            0x3E, 0x12,         // LD A,0x12
            0x77,               // LD (HL),A
            0x06, 0x13,         // LD B,0x13
            0x70,               // LD (HL),B
            0x0E, 0x14,         // LD C,0x14
            0x71,               // LD (HL),C
            0x16, 0x15,         // LD D,0x15
            0x72,               // LD (HL),D
            0x1E, 0x16,         // LD E,0x16
            0x73,               // LD (HL),E
            0x74,               // LD (HL),H
            0x75,               // LD (HL),L
        ];
        cpu.mem.write(0x0000, &prog);

        assert!(10 == cpu.step()); assert!(0x1000 == cpu.r16(HL));
        assert!(7  == cpu.step()); assert!(0x12 == cpu.reg[A]);
        assert!(7  == cpu.step()); assert!(0x12 == cpu.mem.r8(0x1000));
        assert!(7  == cpu.step()); assert!(0x13 == cpu.reg[B]);
        assert!(7  == cpu.step()); assert!(0x13 == cpu.mem.r8(0x1000));
        assert!(7  == cpu.step()); assert!(0x14 == cpu.reg[C]);
        assert!(7  == cpu.step()); assert!(0x14 == cpu.mem.r8(0x1000));
        assert!(7  == cpu.step()); assert!(0x15 == cpu.reg[D]);
        assert!(7  == cpu.step()); assert!(0x15 == cpu.mem.r8(0x1000));
        assert!(7  == cpu.step()); assert!(0x16 == cpu.reg[E]);
        assert!(7  == cpu.step()); assert!(0x16 == cpu.mem.r8(0x1000));
        assert!(7  == cpu.step()); assert!(0x10 == cpu.mem.r8(0x1000));
        assert!(7  == cpu.step()); assert!(0x00 == cpu.mem.r8(0x1000));
    }

    #[test]
    fn test_inc_dec_ss() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x01, 0x00, 0x00,       // LD BC,0x0000
            0x11, 0xFF, 0xFF,       // LD DE,0xffff
            0x21, 0xFF, 0x00,       // LD HL,0x00ff
            0x31, 0x11, 0x11,       // LD SP,0x1111
            0x0B,                   // DEC BC
            0x03,                   // INC BC
            0x13,                   // INC DE
            0x1B,                   // DEC DE
            0x23,                   // INC HL
            0x2B,                   // DEC HL
            0x33,                   // INC SP
            0x3B,                   // DEC SP
        ];
        cpu.mem.write(0x0000, &prog);

        for _ in 0..4 {
            cpu.step();
        }
        assert!(6 == cpu.step()); assert!(0xFFFF == cpu.r16(BC));
        assert!(6 == cpu.step()); assert!(0x0000 == cpu.r16(BC));
        assert!(6 == cpu.step()); assert!(0x0000 == cpu.r16(DE));
        assert!(6 == cpu.step()); assert!(0xFFFF == cpu.r16(DE));
        assert!(6 == cpu.step()); assert!(0x0100 == cpu.r16(HL));
        assert!(6 == cpu.step()); assert!(0x00FF == cpu.r16(HL));
        assert!(6 == cpu.step()); assert!(0x1112 == cpu.r16(SP));
        assert!(6 == cpu.step()); assert!(0x1111 == cpu.r16(SP));
    }

    #[test]
    fn test_ld_a_ibcdenn() {
        let mut cpu = rz80::CPU::new();

        let data = [ 0x11, 0x22, 0x33];
        cpu.mem.write(0x1000, &data);

        let prog = [
            0x01, 0x00, 0x10,       // LD BC,0x1000
            0x11, 0x01, 0x10,       // LD DE,0x1001
            0x0A,                   // LD A,(BC)
            0x1A,                   // LD A,(DE)
            0x3A, 0x02, 0x10,       // LD A,(0x1002)
        ];
        cpu.mem.write(0x0000, &prog);

        assert!(10 == cpu.step()); assert!(0x1000 == cpu.r16(BC));
        assert!(10 == cpu.step()); assert!(0x1001 == cpu.r16(DE));
        assert!(7  == cpu.step()); assert!(0x11 == cpu.reg[A]);
        assert!(7  == cpu.step()); assert!(0x22 == cpu.reg[A]);
        assert!(13 == cpu.step()); assert!(0x33 == cpu.reg[A]);
    }

    #[test]
    fn test_ld_ibcdenn_a() {
        let mut cpu = rz80::CPU::new();

        let prog = [
            0x01, 0x00, 0x10,   // LD BC,0x1000
            0x11, 0x01, 0x10,   // LD DE,0x1001
            0x3E, 0x77,         // LD A,0x77
            0x02,               // LD (BC),A
            0x12,               // LD (DE),A
            0x32, 0x02, 0x10,   // LD (0x1002),A
        ];
        cpu.mem.write(0x0000, &prog);

        assert!(10 == cpu.step()); assert!(0x1000 == cpu.r16(BC));
        assert!(10 == cpu.step()); assert!(0x1001 == cpu.r16(DE));
        assert!(7  == cpu.step()); assert!(0x77 == cpu.reg[A]);
        assert!(7  == cpu.step()); assert!(0x77 == cpu.mem.r8(0x1000));
        assert!(7  == cpu.step()); assert!(0x77 == cpu.mem.r8(0x1001));
        assert!(13 == cpu.step()); assert!(0x77 == cpu.mem.r8(0x1002));
    }

    #[test]
    fn test_rlca_rla_rrca_rra() {
        let mut cpu = rz80::CPU::new();

        let prog = [
            0x3E, 0xA0,     // LD A,0xA0
            0x07,           // RLCA
            0x07,           // RLCA
            0x0F,           // RRCA
            0x0F,           // RRCA
            0x17,           // RLA
            0x17,           // RLA
            0x1F,           // RRA
            0x1F,           // RRA
        ];
        cpu.mem.write(0x0000, &prog);

        cpu.reg[F] = 0xFF;
        assert!(7==cpu.step()); assert!(0xA0 == cpu.reg[A]);
        assert!(4==cpu.step()); assert!(0x41 == cpu.reg[A]); 
        assert!(4==cpu.step()); assert!(0x82 == cpu.reg[A]); 
        assert!(4==cpu.step()); assert!(0x41 == cpu.reg[A]); 
        assert!(4==cpu.step()); assert!(0xA0 == cpu.reg[A]); 
        assert!(4==cpu.step()); assert!(0x41 == cpu.reg[A]); 
        assert!(4==cpu.step()); assert!(0x83 == cpu.reg[A]); 
        assert!(4==cpu.step()); assert!(0x41 == cpu.reg[A]); 
        assert!(4==cpu.step()); assert!(0xA0 == cpu.reg[A]);      
    }

    #[test]
    fn test_daa() {
        let mut cpu = rz80::CPU::new();

        let prog = [
            0x3E, 0x15,     // LD A,0x15
            0x06, 0x27,     // LD B,0x27
            0x80,           // ADD A,B
            0x27,           // DAA
            0x90,           // SUB B
            0x27,           // DAA
            0x3E, 0x90,     // LD A,0x90
            0x06, 0x15,     // LD B,0x15
            0x80,           // ADD A,B
            0x27,           // DAA
            0x90,           // SUB B
            0x27,           // DAA
        ];
        cpu.mem.write(0x0000, &prog);

        assert!(7==cpu.step()); assert!(0x15 == cpu.reg[A]);
        assert!(7==cpu.step()); assert!(0x27 == cpu.reg[B]);
        assert!(4==cpu.step()); assert!(0x3C == cpu.reg[A]); assert!(flags(&cpu, 0));
        assert!(4==cpu.step()); assert!(0x42 == cpu.reg[A]); assert!(flags(&cpu, HF|PF));
        assert!(4==cpu.step()); assert!(0x1B == cpu.reg[A]); assert!(flags(&cpu, HF|NF));
        assert!(4==cpu.step()); assert!(0x15 == cpu.reg[A]); assert!(flags(&cpu, NF));
        assert!(7==cpu.step()); assert!(0x90 == cpu.reg[A]); assert!(flags(&cpu, NF));
        assert!(7==cpu.step()); assert!(0x15 == cpu.reg[B]); assert!(flags(&cpu, NF));
        assert!(4==cpu.step()); assert!(0xA5 == cpu.reg[A]); assert!(flags(&cpu, SF));
        assert!(4==cpu.step()); assert!(0x05 == cpu.reg[A]); assert!(flags(&cpu, PF|CF));
        assert!(4==cpu.step()); assert!(0xF0 == cpu.reg[A]); assert!(flags(&cpu, SF|NF|CF));
        assert!(4==cpu.step()); assert!(0x90 == cpu.reg[A]); assert!(flags(&cpu, SF|PF|NF|CF));
    }

    #[test]
    fn test_cpl() {
        let mut cpu = rz80::CPU::new();

        let prog = [
            0x97,           // SUB A
            0x2F,           // CPL
            0x2F,           // CPL
            0xC6, 0xAA,     // ADD A,0xAA
            0x2F,           // CPL
            0x2F,           // CPL
        ];
        cpu.mem.write(0x0000, &prog);

        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|NF));
        assert!(4==cpu.step()); assert!(0xFF == cpu.reg[A]); assert!(flags(&cpu, ZF|HF|NF));
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|HF|NF));
        assert!(7==cpu.step()); assert!(0xAA == cpu.reg[A]); assert!(flags(&cpu, SF));
        assert!(4==cpu.step()); assert!(0x55 == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF));
        assert!(4==cpu.step()); assert!(0xAA == cpu.reg[A]); assert!(flags(&cpu, SF|HF|NF));
    }

    #[test]
    fn test_ccf_scf() {
        let mut cpu = rz80::CPU::new();

        let prog = [
            0x97,           // SUB A
            0x37,           // SCF
            0x3F,           // CCF
            0xD6, 0xCC,     // SUB 0xCC
            0x3F,           // CCF
            0x37,           // SCF
        ];
        cpu.mem.write(0x0000, &prog);

        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|NF));
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|CF));
        assert!(4==cpu.step()); assert!(0x00 == cpu.reg[A]); assert!(flags(&cpu, ZF|HF));
        assert!(7==cpu.step()); assert!(0x34 == cpu.reg[A]); assert!(flags(&cpu, HF|NF|CF)); 
        assert!(4==cpu.step()); assert!(0x34 == cpu.reg[A]); assert!(flags(&cpu, HF));
        assert!(4==cpu.step()); assert!(0x34 == cpu.reg[A]); assert!(flags(&cpu, CF));
    }

    #[test]
    fn test_call_ret() {
        let mut cpu = rz80::CPU::new();

        let prog = [
            0xCD, 0x0A, 0x02,   // CALL l0
            0xCD, 0x0A, 0x02,   // CALL l0
            0xC9,               // l0: RET
        ];
        cpu.mem.write(0x0204, &prog);
        cpu.pc = 0x0204;

        assert!(17 == cpu.step());
        assert!(0x020A == cpu.pc);
        assert!(0xFFFE == cpu.r16(SP));
        assert!(0x0207 == cpu.mem.r16(0xFFFE));
        assert!(10 == cpu.step());
        assert!(0x0207 == cpu.pc);
        assert!(0x0000 == cpu.r16(SP));
        assert!(17 == cpu.step());
        assert!(0x020A == cpu.pc);
        assert!(0xFFFE == cpu.r16(SP));
        assert!(0x020A == cpu.mem.r16(0xFFFE));
        assert!(10 == cpu.step());
        assert!(0x020A == cpu.pc);
        assert!(0x0000 == cpu.r16(SP));
    }

    #[test]
    fn test_call_cc_ret_cc() {
        let mut cpu = rz80::CPU::new();

        let prog = [
			0x97,               //      SUB A
			0xC4, 0x29, 0x02,   //      CALL NZ,l0
			0xCC, 0x29, 0x02,   //      CALL Z,l0
			0xC6, 0x01,         //      ADD A,0x01
			0xCC, 0x2B, 0x02,   //      CALL Z,l1
			0xC4, 0x2B, 0x02,   //      CALL NZ,l1
			0x07,               //      RLCA
			0xEC, 0x2D, 0x02,   //      CALL PE,l2
			0xE4, 0x2D, 0x02,   //      CALL PO,l2
			0xD6, 0x03,         //      SUB 0x03
			0xF4, 0x2F, 0x02,   //      CALL P,l3
			0xFC, 0x2F, 0x02,   //      CALL M,l3
			0xD4, 0x31, 0x02,   //      CALL NC,l4
			0xDC, 0x31, 0x02,   //      CALL C,l4
			0xC9,               //      RET
			0xC0,               // l0:  RET NZ
			0xC8,               //      RET Z
			0xC8,               // l1:  RET Z
			0xC0,               //      RET NZ
			0xE8,               // l2:  RET PE
			0xE0,               //      RET PO
			0xF0,               // l3:  RET P
			0xF8,               //      RET M
			0xD0,               // l4:  RET NC
			0xD8,               //      RET C<Paste>        
        ];
		cpu.mem.write(0x0204, &prog);
		cpu.pc = 0x0204;
		cpu.w16(SP, 0x0100);

        assert!(4 ==cpu.step()); assert!(0x00 == cpu.reg[A]);
        assert!(10==cpu.step()); assert!(0x0208 == cpu.pc);
        assert!(17==cpu.step()); assert!(0x0229 == cpu.pc);
        assert!(5 ==cpu.step()); assert!(0x022A == cpu.pc);
        assert!(11==cpu.step()); assert!(0x020B == cpu.pc);
        assert!(7 ==cpu.step()); assert!(0x01 == cpu.reg[A]);
        assert!(10==cpu.step()); assert!(0x0210 == cpu.pc);
        assert!(17==cpu.step()); assert!(0x022B == cpu.pc);
        assert!(5 ==cpu.step()); assert!(0x022C == cpu.pc);
        assert!(11==cpu.step()); assert!(0x0213 == cpu.pc);
        assert!(4 ==cpu.step()); assert!(0x02 == cpu.reg[A]);
        assert!(10==cpu.step()); assert!(0x0217 == cpu.pc);
        assert!(17==cpu.step()); assert!(0x022D == cpu.pc);
        assert!(5 ==cpu.step()); assert!(0x022E == cpu.pc);
        assert!(11==cpu.step()); assert!(0x021A == cpu.pc);
        assert!(7 ==cpu.step()); assert!(0xFF == cpu.reg[A]);
        assert!(10==cpu.step()); assert!(0x021F == cpu.pc);
        assert!(17==cpu.step()); assert!(0x022F == cpu.pc);
        assert!(5 ==cpu.step()); assert!(0x0230 == cpu.pc);
        assert!(11==cpu.step()); assert!(0x0222 == cpu.pc);
        assert!(10==cpu.step()); assert!(0x0225 == cpu.pc);
        assert!(17==cpu.step()); assert!(0x0231 == cpu.pc);
        assert!(5 ==cpu.step()); assert!(0x0232 == cpu.pc);
        assert!(11==cpu.step()); assert!(0x0228 == cpu.pc);
    }

    #[test]
    fn test_halt() {
        let mut cpu = rz80::CPU::new();

        let prog = [
            0x76,       // HALT
        ];
        cpu.mem.write(0x0000, &prog);

        assert!(4==cpu.step()); assert!(0x0000 == cpu.pc); assert!(cpu.halt);
        assert!(4==cpu.step()); assert!(0x0000 == cpu.pc); assert!(cpu.halt);
        assert!(4==cpu.step()); assert!(0x0000 == cpu.pc); assert!(cpu.halt);
      
    }
}

    

