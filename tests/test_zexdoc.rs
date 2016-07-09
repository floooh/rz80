/*
extern crate rz80;

#[cfg(test)]
mod test_zexdoc {
    use rz80;
    use rz80::C as C;
    use rz80::E as E;
    use rz80::DE as DE;
    use rz80::SP as SP;
    
    static ZEXDOC: &'static [u8] = include_bytes!("zexdoc.com");

    // emulates a CP/M BDOS call, only what's needed by ZEX
    fn cpm_bdos(cpu: &mut rz80::CPU) {
        match cpu.reg[C] {
            2 => {
                // output a character
                print!("{}", cpu.reg[E] as u8 as char);
            },
            9 => {
                // output a string
                let mut addr = cpu.r16_i(DE);
                loop {
                    let c = cpu.mem.r8(addr) as u8 as char;
                    addr = (addr + 1) & 0xFFFF;
                    if c != '$' {
                        print!("{}", c);
                    }
                    else {
                        break;
                    }
                }
            },
            _ => {
                panic!("Unknown CP/M call {}!", cpu.reg[C]);
            }
        }
        // emulate a RET
        let sp = cpu.r16_i(SP);
        cpu.pc = cpu.mem.r16(sp);
        cpu.w16_i(SP, sp + 2);
    }

    #[test]
    #[ignore]
    fn test_zexdoc() {
        let mut cpu = rz80::CPU::new();
        cpu.mem.write(0x0100, &ZEXDOC);
        cpu.w16_i(SP, 0xF000);
        cpu.pc = 0x0100;
        loop {
            cpu.step();
            if cpu.pc == 0x0005 {
                // emulated CP/M BDOS call
                cpm_bdos(&mut cpu);
            }
            else if cpu.pc == 0x0000 {
                break;
            }
        }
    }
}
*/
