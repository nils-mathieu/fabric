//! This module provides way to describe the data that's supposed to be globally accessible to
//! all userspace processes (as well as the kernel) running on a Fabric OS system.
//!
//! Specifically, an instance of [`PublicData`] is mapped in every userspace process and is
//! freely accessible.

mod framebuffer;

pub use self::framebuffer::*;

/// An instance of this structure is mapped in the address space of all processes.
#[repr(C)]
pub struct PublicData {
    /// The framebuffers available to the system.
    pub framebuffers: *const Framebuffer,
    /// The total number of available framebuffers.
    pub framebuffer_count: usize,
}

impl PublicData {
    /// Returns the list of all framebuffers.
    #[inline(always)]
    pub fn framebuffers(&self) -> &[Framebuffer] {
        unsafe { core::slice::from_raw_parts(self.framebuffers, self.framebuffer_count) }
    }
}

/// Returns the global [`PublicData`] instance.
#[cfg(feature = "userland")]
#[inline(always)]
pub fn get() -> &'static PublicData {
    extern "C" {
        static fabric_public_data: PublicData;
    }

    // SAFETY:
    //  Accessing the public data is always safe.
    unsafe { &fabric_public_data }
}
