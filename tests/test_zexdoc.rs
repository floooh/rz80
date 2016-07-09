extern crate rz80;

#[cfg(test)]
mod test_zexdoc {
    use rz80;
    
    static ZEXDOC: &'static [u8] = include_bytes!("zexdoc.com");

    // emulates a CP/M BDOS call, only what's needed by ZEX
    fn cpm_bdos(cpu: &mut rz80::CPU) {
        match cpu.reg.c() {
            2 => {
                // output a character
                print!("{}", cpu.reg.e() as u8 as char);
            },
            9 => {
                // output a string
                let mut addr = cpu.reg.de();
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
                panic!("Unknown CP/M call {}!", cpu.reg.c());
            }
        }
        // emulate a RET
        let sp = cpu.reg.sp();
        cpu.reg.set_pc(cpu.mem.r16(sp));
        cpu.reg.set_sp(sp + 2);
    }

    #[test]
    #[ignore]
    fn test_zexdoc() {
        let mut cpu = rz80::CPU::new();
        cpu.mem.write(0x0100, &ZEXDOC);
        cpu.reg.set_sp(0xF000);
        cpu.reg.set_pc(0x0100);
        loop {
            cpu.step();
            match cpu.reg.pc() {
                0x0005 => { cpm_bdos(&mut cpu); },  // emulated CP/M BDOS call
                0x0000 => { break; },
                _ => { },
            }
        }
    }
}
