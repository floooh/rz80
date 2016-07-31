use std::mem;
use RegT;

const PAGE_SHIFT: usize = 10;   // 1 kByte page size = (1<<10)
const PAGE_SIZE: usize = (1 << PAGE_SHIFT);
const PAGE_MASK: usize = PAGE_SIZE - 1;
const HEAP_SIZE: usize = 128 * PAGE_SIZE;
const NUM_PAGES: usize = (1 << 16) / PAGE_SIZE;
const NUM_LAYERS: usize = 4;

#[derive(Clone,Copy)]
struct Page {
    pub offset: usize, // offset into heap
    pub writable: bool, // true if the page is writable
    pub mapped: bool, // true if currently mapped
}

impl Page {
    /// return a new, unmapped page
    pub fn new() -> Page {
        Page {
            offset: 0,
            writable: false,
            mapped: false,
        }
    }
    /// map page to chunk of heap memory
    pub fn map(&mut self, offset: usize, writable: bool) {
        self.offset = offset;
        self.writable = writable;
        self.mapped = true;
    }
    /// unmap page
    pub fn unmap(&mut self) {
        self.offset = 0;
        self.writable = false;
        self.mapped = false;
    }
}

/// memory access
///
/// The Memory object wraps access to the Z80's 64 KByte
/// address space. All memory access goes through a
/// page table with a page-size of 1 KByte. The page table
/// mapping allows a very simple implementation of
/// bank-switching, which was a popular way in 8-bit computers to
/// manage more than 64 KBytes of memory.
///
/// ## Memory Layers
///
/// Mapped memory is assigned to 1 out of (currently) 4 layers. If
/// 2 memory chunks are mapped to the same CPU address range on
/// different layers, only the memory assigned to the higher-priority
/// layer is visible to the CPU (layer number 0 has the highest
/// priority and layer number 3 the lowest).
///
/// The layer concept is easier to visualize than to describe:
///
/// ```text
///                 +---------------------------------------+
/// LAYER 3         |333333333333333333333333333333333333333|
///                 +-------+---------------+---------------+
/// LAYER 2                 |222222222222222|
///                         +---------------+       +-------+
/// LAYER 1                                         |1111111|
///                               +---------+       +-------+
/// LAYER 0                       |000000000|
///                               +---------+
///                 +-------+-----+---------+-------+-------+
/// CPU VISIBLE:    |3333333|22222|000000000|3333333|1111111|
///                 +-------+-----+---------+-------+-------+
/// ```
///
/// ## The Heap
/// 
/// The Memory class will never keep references to external memory, instead it
/// comes with it's own few hundred KBytes of embedded memory which is used 
/// as 'heap'. A single memory page maps 1 KByte of memory from the Z80
/// address range to 1 KByte of memory somewhere on the embedded heap.
///
/// ## Mapping Memory
/// 
/// This 'maps' a chunk of memory in Z80 address range to a chunk of memory
/// of the same size in the embedded heap on one of the four memory layers.
///
/// The simple form performs the memory mapping but does not copy
/// any data into the mapped memory region:
///
/// ```
/// use rz80::Memory;
/// let mut mem = Memory::new();
///
/// // map 32 KByte at heap address 0x08000 to CPU addr 0x0000 
/// // on layer 0 as writable:
/// mem.map(0, 0x08000, 0x0000, true, 32*1024);
///
/// // map another 32 KByte at heap address 0x10000 to CPU addr 0x8000 
/// // on layer 1 as read-only:
/// mem.map(1, 0x10000, 0x8000, false, 32*1024);
/// ```
///
/// The method **map_bytes()** performs a memory mapping as above,
/// but also copies a range of bytes into the mapped memory. This is
/// useful to initialize the memory with a ROM dump.
///
/// ```
/// use rz80::Memory;
/// let mut mem = Memory::new();
/// let rom = [0xFFu8; 4096];
/// 
/// // assume that 'rom' is a system ROM dump, and map it as read-only to CPU address 0xF000
/// mem.map_bytes(0, 0x00000, 0xF000, false, &rom);
/// ```
///
/// ## Reading and Writing Memory
/// (TODO!)
pub struct Memory {
    /// currently CPU-visible pages
    pages: [Page; NUM_PAGES],
    /// currently mapped layers
    layers: [[Page; NUM_PAGES]; NUM_LAYERS],
    /// 'host' memory
    pub heap: [u8; HEAP_SIZE],
}

impl Memory {
    /// return new, unmapped memory object
    pub fn new() -> Memory {
        Memory {
            pages: [Page::new(); NUM_PAGES],
            layers: [[Page::new(); NUM_PAGES]; NUM_LAYERS],
            heap: [0; HEAP_SIZE],
        }
    }

    /// return new memory object with 64 kByte mapped, writable memory (for testing)
    pub fn new_64k() -> Memory {
        let mut mem = Memory::new();
        mem.map(0, 0, 0, true, (1 << 16));
        mem
    }

    /// map a chunk of uninitialized heap memory to CPU-mapped memory
    pub fn map(&mut self,
               layer: usize,
               heap_offset: usize,
               addr: usize,
               writable: bool,
               size: usize) {
        assert!((size & PAGE_MASK) == 0);
        assert!((addr & PAGE_MASK) == 0);
        let num = size >> PAGE_SHIFT;
        for i in 0..num {
            let map_offset = i * PAGE_SIZE;
            let page_index = ((addr + map_offset) & 0xFFFF) >> PAGE_SHIFT;
            let page = &mut self.layers[layer][page_index];
            page.map(heap_offset + map_offset, writable);
        }
        self.update_mapping();
    }

    /// map a chunk of heap memory, and initialize it
    pub fn map_bytes(&mut self,
                     layer: usize,
                     heap_offset: usize,
                     addr: usize,
                     writable: bool,
                     content: &[u8]) {
        assert!((addr & PAGE_MASK) == 0);
        let size = mem::size_of_val(content);
        assert!((size & PAGE_MASK) == 0);
        self.map(layer, heap_offset, addr, writable, size);
        let dst = &mut self.heap[heap_offset..heap_offset + size];
        dst.clone_from_slice(content);
    }

    /// unmap a chunk heap memory
    pub fn unmap(&mut self, layer: usize, size: usize, addr: usize) {
        assert!((size & PAGE_MASK) == 0);
        assert!((addr & PAGE_MASK) == 0);
        let num = size >> PAGE_SHIFT;
        for i in 0..num {
            let map_offset = i * PAGE_SIZE;
            let page_index = ((addr + map_offset) & 0xFFFF) >> PAGE_SHIFT;
            let page = &mut self.layers[layer][page_index];
            page.unmap();
        }
        self.update_mapping();
    }

    /// unmap all pages in a layer
    pub fn unmap_layer(&mut self, layer: usize) {
        for page in self.layers[layer].iter_mut() {
            page.unmap();
        }
        self.update_mapping();
    }

    /// unmap all pages in all layers
    pub fn unmap_all(&mut self) {
        for layer in self.layers.iter_mut() {
            for page in layer.iter_mut() {
                page.unmap();
            }
        }
        self.update_mapping();
    }

    /// private method to update internal CPU-visible mapping from mapped layers
    fn update_mapping(&mut self) {
        // for each cpu-visible page, find the highest-priority layer
        // which maps this memory range and copy it into the
        // cpu-visible page
        for page_index in 0..NUM_PAGES {
            let mut layer_page: Option<&Page> = None;
            for layer_index in 0..NUM_LAYERS {
                if self.layers[layer_index][page_index].mapped {
                    layer_page = Some(&self.layers[layer_index][page_index]);
                    break;
                }
            }
            match layer_page {
                Some(page) => self.pages[page_index] = *page,
                None => self.pages[page_index].unmap(),
            }
        }
    }

    /// read unsigned byte from 16-bit address
    #[inline(always)]
    pub fn r8(&self, addr: RegT) -> RegT {
        let uaddr = (addr & 0xFFFF) as usize;
        let page = &self.pages[uaddr >> PAGE_SHIFT];
        if page.mapped {
            let heap_offset = page.offset + (uaddr & PAGE_MASK);
            self.heap[heap_offset] as RegT
        } else {
            0xFF
        }
    }

    /// read signed byte from 16-bit address
    #[inline(always)]
    pub fn rs8(&self, addr: RegT) -> RegT {
        let uaddr = (addr & 0xFFFF) as usize;
        let page = &self.pages[uaddr >> PAGE_SHIFT];
        if page.mapped {
            let heap_offset = page.offset + (uaddr & PAGE_MASK);
            self.heap[heap_offset] as i8 as RegT
        } else {
            0xFF
        }
    }

    /// write unsigned byte to 16-bit address
    #[inline(always)]
    pub fn w8(&mut self, addr: RegT, val: RegT) {
        let uaddr = (addr & 0xFFFF) as usize;
        let page = &self.pages[uaddr >> PAGE_SHIFT];
        if page.mapped && page.writable {
            let heap_offset = page.offset + (uaddr & PAGE_MASK);
            self.heap[heap_offset] = val as u8;
        }
    }

    /// write unsigned byte, ignore write-protection flag
    pub fn w8f(&mut self, addr: RegT, val: RegT) {
        let uaddr = (addr & 0xFFFF) as usize;
        let page = &self.pages[uaddr >> PAGE_SHIFT];
        if page.mapped {
            let heap_offset = page.offset + (uaddr & PAGE_MASK);
            self.heap[heap_offset] = val as u8;
        }
    }

    /// read unsigned word from 16-bit address
    #[inline(always)]
    pub fn r16(&self, addr: RegT) -> RegT {
        let l = self.r8(addr);
        let h = self.r8(addr + 1);
        h << 8 | l
    }

    /// write unsigned word to 16-bit address
    #[inline(always)]
    pub fn w16(&mut self, addr: RegT, val: RegT) {
        let l = val & 0xff;
        let h = (val >> 8) & 0xff;
        self.w8(addr, l);
        self.w8(addr + 1, h);
    }

    /// write a whole chunk of memory, ignore write-protection
    pub fn write(&mut self, addr: RegT, data: &[u8]) {
        let mut offset = 0;
        for b in data {
            self.w8f(addr + offset, *b as RegT);
            offset += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_readwrite() {
        let mut mem = Memory::new_64k();
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

    #[test]
    fn mem_map() {
        let mut mem = Memory::new();
        const SIZE: usize = 0x4000;  // 16k
        let x11 = [0x11u8; SIZE];
        let x22 = [0x22u8; SIZE];
        let x33 = [0x33u8; SIZE];
        let x44 = [0x44u8; SIZE];
        mem.map_bytes(0, 0x0000, 0x0000, true, &x11);
        mem.map_bytes(0, 0x4000, 0x4000, true, &x22);
        mem.map_bytes(0, 0x8000, 0x8000, true, &x33);
        mem.map_bytes(0, 0xC000, 0xC000, false, &x44);
        assert!(mem.r8(0x0000) == 0x11);
        assert!(mem.r8(0x4000) == 0x22);
        assert!(mem.r8(0x8000) == 0x33);
        assert!(mem.r8(0xC000) == 0x44);
        assert!(mem.r8(0x3FFF) == 0x11);
        assert!(mem.r8(0x7FFF) == 0x22);
        assert!(mem.r8(0xBFFF) == 0x33);
        assert!(mem.r8(0xFFFF) == 0x44);
        assert!(mem.r16(0x3FFF) == 0x2211);
        assert!(mem.r16(0x7FFF) == 0x3322);
        assert!(mem.r16(0xBFFF) == 0x4433);
        assert!(mem.r16(0xFFFF) == 0x1144);
        mem.w16(0xBFFF, 0x1234);
        assert!(mem.r8(0xBFFF) == 0x34);
        assert!(mem.r8(0xC000) == 0x44);
        mem.unmap(0, 0x4000, SIZE);
        assert!(mem.r8(0x4000) == 0xFF);
        assert!(mem.r8(0x7FFF) == 0xFF);
        assert!(mem.r8(0x3FFF) == 0x11);
        assert!(mem.r8(0x8000) == 0x33);
        mem.w8(0x4000, 0x55);
        assert!(mem.r8(0x4000) == 0xFF);
        mem.w8(0x0000, 0x66);
        assert!(mem.r8(0x0000) == 0x66);
    }

    #[test]
    fn mem_layers() {
        let mut mem = Memory::new();
        const SIZE: usize = 0x8000;  // 32k
        let x11 = [0x11u8; SIZE];
        let x22 = [0x22u8; SIZE];
        let x33 = [0x33u8; SIZE];
        let x44 = [0x44u8; SIZE];
        mem.map_bytes(3, 0x00000, 0x0000, true, &x11);
        mem.map_bytes(2, 0x08000, 0x4000, true, &x22);
        mem.map_bytes(1, 0x10000, 0x8000, true, &x33);
        mem.map_bytes(0, 0x18000, 0xC000, true, &x44);
        assert!(mem.r8(0x0000) == 0x44);    // layer 0 is wrapping around at 0xFFFF
        assert!(mem.r8(0x4000) == 0x22);
        assert!(mem.r8(0x8000) == 0x33);
        assert!(mem.r8(0xC000) == 0x44);
        mem.unmap(0, 0xC000, SIZE);
        assert!(mem.r8(0x0000) == 0x11);
        assert!(mem.r8(0x4000) == 0x22);
        assert!(mem.r8(0x8000) == 0x33);
        assert!(mem.r8(0xC000) == 0x33);
    }
}
