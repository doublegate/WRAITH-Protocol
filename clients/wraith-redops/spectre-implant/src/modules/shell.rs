use alloc::vec::Vec;

pub struct Shell;

impl Shell {
    pub fn exec(&self, _cmd: &str) -> Vec<u8> {
        // Placeholder for shell execution
        // Implementing full pipe I/O in no_std without std::process is complex.
        Vec::from(b"Shell output placeholder".as_slice())
    }
}
