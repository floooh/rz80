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
}

    

