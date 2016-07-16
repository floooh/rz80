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
    fn inp(&mut self, port: RegT) -> RegT { 0 }
    /// CPU writes to I/O port
    fn outp(&mut self, port: RegT, val: RegT) { }
}
