use RegT;

/// system bus trait
/// The system bus must be implemented by the higher level parts
/// of an emulator and is used as central callback facility for the 
/// various Z80 chips. If anything happens in the chips that 
/// need to be communicated to other chips or the higher-level
/// parts of the emulator (such as port I/O), one of the
/// trait functions will be called.
#[allow(unused_variables)]
pub trait Bus {
    /// CPU reads from I/O port
    fn cpu_inp(&mut self, port: RegT) -> RegT { 0 }
    /// CPU writes to I/O port
    fn cpu_outp(&mut self, port: RegT, val: RegT) { }

    /// PIO output callback
    fn pio_outp(&mut self, pio: usize, chn: usize, data: RegT) { }
    /// PIO input callback
    fn pio_inp(&mut self, pio: usize, chn: usize) -> RegT { 0 }
    /// PIO channel rdy line has changed
    fn pio_rdy(&mut self, pio: usize, chn: usize, rdy: bool) { }
    /// PIO interrupt request
    fn pio_int(&mut self, pio: usize, chn: usize, int_vector: RegT) { }
}
