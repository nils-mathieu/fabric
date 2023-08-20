//! This module exports some functions defined by the `compiler_builtins` crate.
//!
//! We can't use the crate directly because `compiler_builtins` is a private dependency of `core`,
//! but the symbols are still available globally.

use core::ffi::c_char;

extern "C" {
    pub fn strlen(s: *const c_char) -> usize;
}
