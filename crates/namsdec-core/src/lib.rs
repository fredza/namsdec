pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod cpu;
pub mod digest;
pub mod flags;
pub mod memory;
pub mod registers;

pub use cpu::Cpu;
pub use flags::{RFlags, Width, add_with_flags, logic_flags, sub_with_flags};
pub use memory::{Access, MemError, Memory, Perms};
pub use registers::{Precision, Reg64, RegMask, RegXmm, Registers, Rounding, X87};
