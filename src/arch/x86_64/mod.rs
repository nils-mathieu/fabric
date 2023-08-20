//! Structures, functions and constants for the **x86_64** architecture.
//!
//! # Modules
//!
//! The following modules are defined, providing documentation for the various relevant parts of
//! the code base for the **x86_64** architecture:
//!
//! - [`cpu`]: CPU-specific code. Not including scheduling.
//! - [`mem`]: Physical memory management.
//! - [`serial`]: Serial port driver.

use fabric_sys::x86_64::public::PublicData;

#[path = "boot/limine/mod.rs"]
mod limine;

mod cpu;
mod instr;
mod kernel_stack;
mod mem;
mod process;
mod public;
mod raw;
mod scheduler;
mod serial;
mod syscall;

/// Disables interrupts and halts the CPU forever.
pub fn die() -> ! {
    instr::cli();

    // The HLT instruction shouldn't actually return, but in case it spuriously does, we add a
    // jump to continue halting the CPU.
    loop {
        instr::hlt();
    }
}

/// Returns the address of the beginning of the kernel image.
///
/// This value is computed by the linker.
#[inline(always)]
pub fn image_begin() -> usize {
    extern "C" {
        static __fabric_image_begin: u8;
    }

    // SAFETY:
    //  We just taking the address of the symbol, without creating a reference to it. This is
    //  always safe.
    unsafe { core::ptr::addr_of!(__fabric_image_begin) as usize }
}

/// Returns the address of the end of the kernel image.
///
/// This vlaue is computed by the linker.
#[inline(always)]
pub fn image_end() -> usize {
    extern "C" {
        static __fabric_image_end: u8;
    }

    // SAFETY:
    //  We just taking the address of the symbol, without creating a reference to it. This is
    //  always safe.
    unsafe { core::ptr::addr_of!(__fabric_image_end) as usize }
}

/// Returns the virtual address of the global [`PublicData`] instance.
#[inline(always)]
pub fn public_data_address() -> usize {
    extern "C" {
        static __fabric_public_data_address: PublicData;
    }

    // SAFETY:
    //  We just taking the address of the symbol, without creating a reference to it. This is
    //  always safe.
    unsafe { core::ptr::addr_of!(__fabric_public_data_address) as usize }
}
