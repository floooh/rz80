extern crate rz80;

#[cfg(test)]
mod test_opdodes {
    use rz80;

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

        cpu.reg[rz80::A] = 0x12;
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[rz80::B]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[rz80::C]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[rz80::D]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[rz80::E]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[rz80::H]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[rz80::L]);
        assert!(4 == cpu.step()); assert!(0x12 == cpu.reg[rz80::A]);
        cpu.reg[rz80::B] = 0x13;
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[rz80::C]);
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[rz80::D]);
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[rz80::E]);
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[rz80::H]);
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[rz80::L]);
        assert!(4 == cpu.step()); assert!(0x13 == cpu.reg[rz80::A]);
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

        cpu.reg[rz80::A] = 0x33;
        cpu.w16(rz80::HL, 0x1000);
        cpu.pc = 0x0100;
        assert!(7 == cpu.step()); assert!(0x33 == cpu.mem.r8(0x1000));
        assert!(7 == cpu.step()); assert!(0x33 == cpu.reg[rz80::B]);
        assert!(7 == cpu.step()); assert!(0x33 == cpu.reg[rz80::C]);
        assert!(7 == cpu.step()); assert!(0x33 == cpu.reg[rz80::D]);
        assert!(7 == cpu.step()); assert!(0x33 == cpu.reg[rz80::E]);
        assert!(7 == cpu.step()); assert!(0x33 == cpu.reg[rz80::H]);
    }

    #[test]
    fn test_add_r() {
        let mut cpu = rz80::CPU::new();
        let prog = [
            0x87,       // ADD A,A
            0x80,       // ADD A,B
            0x81,       // ADD A,C
        ];
        cpu.mem.write(0x0000, &prog);

        cpu.reg[rz80::A] = 0x0F;
        cpu.reg[rz80::B] = 0xE0;
        assert!(4 == cpu.step()); assert!(0x1E == cpu.reg[rz80::A]);
        assert!(4 == cpu.step()); assert!(0xFE == cpu.reg[rz80::A]);
        cpu.reg[rz80::A] = 0x81;
        cpu.reg[rz80::C] = 0x80;
        assert!(4 == cpu.step()); assert!(0x01 == cpu.reg[rz80::A]);
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

        assert!(7  == cpu.step()); assert!(0x03 == cpu.reg[rz80::B]);
        assert!(4  == cpu.step()); assert!(0x00 == cpu.reg[rz80::A]);
        assert!(4  == cpu.step()); assert!(0x01 == cpu.reg[rz80::A]);
        assert!(13 == cpu.step()); assert!(0x02 == cpu.reg[rz80::B]); assert!(0x0207 == cpu.pc);
        assert!(4  == cpu.step()); assert!(0x02 == cpu.reg[rz80::A]);
        assert!(13 == cpu.step()); assert!(0x01 == cpu.reg[rz80::B]); assert!(0x0207 == cpu.pc);
        assert!(4  == cpu.step()); assert!(0x03 == cpu.reg[rz80::A]);
        assert!(8  == cpu.step()); assert!(0x00 == cpu.reg[rz80::B]); assert!(0x020A == cpu.pc);
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

        assert!(4  == cpu.step()); assert!(0x00 == cpu.reg[rz80::A]);
        assert!(7  == cpu.step()); assert!(0x0207 == cpu.pc);
        assert!(12 == cpu.step()); assert!(0x020A == cpu.pc);
        assert!(7  == cpu.step()); assert!(0x01 == cpu.reg[rz80::A]);
        assert!(7  == cpu.step()); assert!(0x020E == cpu.pc);
        assert!(12 == cpu.step()); assert!(0x0211 == cpu.pc);
        assert!(7  == cpu.step()); assert!(0xFE == cpu.reg[rz80::A]);
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

        assert!(10 == cpu.step()); assert!(0x1000 == cpu.r16(rz80::HL));
        assert!(7  == cpu.step()); assert!(0x12 == cpu.reg[rz80::A]);
        assert!(7  == cpu.step()); assert!(0x12 == cpu.mem.r8(0x1000));
        assert!(7  == cpu.step()); assert!(0x13 == cpu.reg[rz80::B]);
        assert!(7  == cpu.step()); assert!(0x13 == cpu.mem.r8(0x1000));
        assert!(7  == cpu.step()); assert!(0x14 == cpu.reg[rz80::C]);
        assert!(7  == cpu.step()); assert!(0x14 == cpu.mem.r8(0x1000));
        assert!(7  == cpu.step()); assert!(0x15 == cpu.reg[rz80::D]);
        assert!(7  == cpu.step()); assert!(0x15 == cpu.mem.r8(0x1000));
        assert!(7  == cpu.step()); assert!(0x16 == cpu.reg[rz80::E]);
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
        assert!(6 == cpu.step()); assert!(0xFFFF == cpu.r16(rz80::BC));
        assert!(6 == cpu.step()); assert!(0x0000 == cpu.r16(rz80::BC));
        assert!(6 == cpu.step()); assert!(0x0000 == cpu.r16(rz80::DE));
        assert!(6 == cpu.step()); assert!(0xFFFF == cpu.r16(rz80::DE));
        assert!(6 == cpu.step()); assert!(0x0100 == cpu.r16(rz80::HL));
        assert!(6 == cpu.step()); assert!(0x00FF == cpu.r16(rz80::HL));
        assert!(6 == cpu.step()); assert!(0x1112 == cpu.r16(rz80::SP));
        assert!(6 == cpu.step()); assert!(0x1111 == cpu.r16(rz80::SP));
    }
}

    

