//! Common types used by both the Fabric kernel and userspace programs.
//!
//! # Portability
//!
//! Because the Fabric kernel is not portable by design, this crate is also not portable.
//! Architecture-specific code is placed in their respective modules.

#![no_std]

#[cfg(target_arch = "x86_64")]
#[path = "arch/x86_64/mod.rs"]
pub mod x86_64;

mod process;
mod sys_result;

pub use self::process::*;
pub use self::sys_result::*;
