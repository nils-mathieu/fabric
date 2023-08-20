//! Provides miscellaneous utility functions and types.

mod epoch_mutex;
mod fmt;

pub use self::epoch_mutex::*;
pub use self::fmt::*;

/// Aligns the given value to the next page boundary (4 KiB).
#[inline(always)]
pub fn align_page_up(x: usize) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        (x + 0xfff) & !0xfff
    }
}

/// Aligns the given value to the previous page boundary (4 KiB).
#[inline(always)]
pub fn align_page_down(x: usize) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        x & !0xfff
    }
}
