use RegT;
const SIZE : usize = 1<<16;

/// memory access (simplified, no memory mapping or bank switching)
pub struct Memory {
    buf: [u8; SIZE]
}

impl Memory {
    /// return new, cleared Memory object 
    pub fn new() -> Memory {
        Memory {
            buf: [ 0; SIZE ]
        }
    }

    /// clear memory to 0
    pub fn clear(&mut self) {
        self.buf = [ 0; SIZE ];
    }

    /// read unsigned byte from 16-bit address
    pub fn r8(&self, addr: RegT) -> RegT {
        self.buf[(addr & 0xFFFF) as usize] as RegT
    }

    /// write unsigned byte to 16-bit address
    pub fn w8(&mut self, addr: RegT, val: RegT) {
        self.buf[(addr & 0xFFFF) as usize] = val as u8;
    }

    /// read unsigned word from 16-bit address
    pub fn r16(&self, addr: RegT) -> RegT {
        let l = self.r8(addr);
        let h = self.r8(addr + 1);
        h<<8 | l
    }

    /// write unsigned word to 16-bit address
    pub fn w16(&mut self, addr: RegT, val: RegT) {
        let l = val & 0xff;
        let h = (val >> 8) & 0xff;
        self.w8(addr, l);
        self.w8(addr + 1, h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_clear() {
        let mem = Memory::new();
        assert!(mem.r8(0x1234) == 0);
        assert!(mem.r16(0x0) == 0);
        assert!(mem.r16(0xFFFF) == 0);
        let addr = 0xFFFF;
        assert!(mem.r16(addr) == 0);
    }

    #[test]
    fn mem_readwrite() {
        let mut mem = Memory::new();
        mem.w8(0x1234, 0x12);
        assert!(mem.r8(0x1234) == 0x12);

        mem.w8(0x2345, 0x32);
        assert!(mem.r8(0x2345) == 0x32);

        mem.w16(0x1000, 0x1234);
        assert!(mem.r16(0x1000) == 0x1234);
        assert!(mem.r8(0x1000) == 0x34);
        assert!(mem.r8(0x1001) == 0x12);
        
        mem.w16(0xFFFF, 0x2233);
        assert!(mem.r16(0xFFFF) == 0x2233);
        assert!(mem.r8(0xFFFF) == 0x33);
        assert!(mem.r8(0x0000) == 0x22);
    }
}
