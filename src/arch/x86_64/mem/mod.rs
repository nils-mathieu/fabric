//! Provides way to manipulate the physical-to-virtual memory mappings.
//!
//! # Map of the virtual memory
//!
//! The virtual memory is mainly divided into two parts:
//!
//! - The **lower half** (addresses `0x00000000_00000000` to `0x00007FFF_FFFFFFFF`) is used by
//!   userland processes. Processes are free to use the memory as they wish, and the kernel will
//!   not interfere with their memory management.
//!
//! - The **higher half** (addresses `0xFFFF8000_00000000` to `0xFFFFFFFF_FFFFFFFF`) is used by
//!   the kernel. Userspace pointers will never be able to point to this region of memory, and
//!   system calls should always check that pointers passed by users are part of the lower half.
//!
//! Note that the page table is set up in the [`crate::x86_64::cpu::paging`] module.

/// The maximum amount of physical memory supported by the kernel.
///
/// This is currently (and kinda arbitrarily) set to 1 TiB.
pub const MAX_PHYSICAL_MEMORY: usize = 1024 * 1024 * 1024 * 1024;

/// The size of a physical page.
pub const PAGE_SIZE: usize = 4096;

/// The last address of the lower half of the address space.
///
/// This is technically a pointer to the last byte that may be addressed by a pointer in userspace.
// pub const USERSLAND_STOP: usize = 0x00007FFF_FFFFFFFF;

/// The offset between physical addresses and virtual addresses in the higher half.
pub const HHDM_OFFSET: usize = 0xFFFF8000_00000000;

/// The first value that is not part of the virtual address space of userland processes.
pub const USER_TOP: usize = 0x00007FFF_FFFFFFFF;

/// Indicates that the allocator cannot allocate for the requested amount of memory.
#[derive(Debug, Clone, Copy)]
pub struct OutOfMemory;

mod boot_allocator;
mod memory_tracker;

pub use self::boot_allocator::*;
pub use self::memory_tracker::*;
