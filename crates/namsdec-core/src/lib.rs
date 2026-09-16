pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod cpu;
pub mod flags;
pub mod memory;
pub mod registers;

pub use cpu::Cpu;
pub use flags::{RFlags, Width, add_with_flags, logic_flags, sub_with_flags};
pub use memory::{MemError, Memory};
pub use registers::{Reg64, RegXmm, Registers, X87};
