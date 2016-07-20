extern crate rz80;
extern crate time;

#[cfg(test)]
mod test_zex {
    use time::PreciseTime;
    use rz80;
    
    static ZEXDOC: &'static [u8] = include_bytes!("zexdoc.com");
    static ZEXALL: &'static [u8] = include_bytes!("zexall.com");

    struct DummyBus { }
    impl rz80::Bus for DummyBus { }

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
                    let c = cpu.mem.r8(addr) as u8;
                    addr = (addr + 1) & 0xFFFF;
                    if c != b'$' {
                        print!("{}", c as char);
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
        cpu.ret();
    }

    fn run_test(prog: &[u8]) -> (i64, i64) {
        let mut num_ops = 0;
        let mut num_cycles = 0;
        let mut cpu = rz80::CPU::new_64k();
        let mut bus = DummyBus { };
        cpu.mem.write(0x0100, prog);
        cpu.reg.set_sp(0xF000);
        cpu.reg.set_pc(0x0100);
        loop {
            num_ops += 1;
            num_cycles += cpu.step(&mut bus);
            match cpu.reg.pc() {
                0x0005 => { cpm_bdos(&mut cpu); },  // emulated CP/M BDOS call
                0x0000 => { break; },
                _ => { },
            }
        }
        (num_ops, num_cycles)
    }

    fn test_zexdoc() {
        println!(">>> RUNNING ZEXDOC");

        let start = PreciseTime::now();
        let (num_ops, num_cycles) = run_test(&ZEXDOC);
        let end = PreciseTime::now();
        let ms = start.to(end).num_milliseconds();
        let mips = (num_ops / ms)/1000;
        let mhz  = (num_cycles / ms)/1000;
        
        println!("\n\nops: {}, cycles: {}, duration: {}ms", num_ops, num_cycles, ms);
        println!("mips: {}, MHz: {}\n\n", mips, mhz);
    }
    
    fn test_zexall() {
        println!(">>> RUNNING ZEXALL");

        let start = PreciseTime::now();
        let (num_ops, num_cycles) = run_test(&ZEXALL);
        let end = PreciseTime::now();
        let ms = start.to(end).num_milliseconds();
        let mips = (num_ops / ms)/1000;
        let mhz  = (num_cycles / ms)/1000;
        
        println!("\n\nops: {}, cycles: {}, duration: {}ms", num_ops, num_cycles, ms);
        println!("mips: {}, MHz: {}", mips, mhz);
    }
    
    #[test]
    #[ignore]
    fn test_zex() {
        // have 1 test function run both sub-tests, we don't want to
        // run them in parallel
        test_zexdoc();
        test_zexall();
    }
}
