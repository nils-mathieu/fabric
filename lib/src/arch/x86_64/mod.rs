//! Defines types and constants for the x86_64 architecture.

use bitflags::bitflags;

#[cfg(feature = "userland")]
use crate::{ProcessId, SysResult};

#[cfg(feature = "userland")]
pub mod raw;

pub mod public;

/// An enumeration of all valid system call numbers on **x86_64**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(usize)]
pub enum Syscall {
    Terminate,
    MapMemory,
    UnmapMemory,
    AcquireFramebuffer,
    ReleaseFramebuffer,
}

bitflags! {
    /// Flags used for memory mapping in system calls.
    #[derive(Debug, Clone, Copy)]
    pub struct MapFlags: usize {
        /// Whether the pages are writable.
        ///
        /// When this flag is set, the mapping is read-write. Otherwise, it is read-only.
        const WRITABLE = 1 << 0;

        /// Whether the pages are executable.alloc
        ///
        /// When this flag is set, the mapping can be executed. Otherwise, it is not executable and
        /// attempting to execute code in it will result in a page fault.
        const EXECUTABLE = 1 << 1;
    }
}

/// Performs the `terminate` system call on the current process.
///
/// # Returns
///
/// This function never returns and may never fail.
#[cfg(feature = "userland")]
#[inline(always)]
pub fn terminate_self() -> ! {
    unsafe {
        raw::syscall1(Syscall::Terminate as usize, 0);
        core::hint::unreachable_unchecked();
    }
}

/// Maps some physical memory into the virtual address space of the specified process.
///
/// # Arguments
///
/// - `process_id` is the ID of the process to map to the memory for. 0 indicates the current
///   process.
///
/// - `virtual_address` is the virtual address to map the memory to. This must be aligned to a
///   page boundary.
///
/// - `length` is the length of the memory region to map. This must be aligned to a page boundary.
///
/// - `flags` is a bitfield of flags that control the mapping. The supported flags are defined
///   in [`MapFlags`].
///
/// # Returns
///
/// On success, this system call returns 0.
///
/// # Errors
///
/// [`SysResult::INVALID_VALUE`] is returned if:
///
/// - The target address is not aligned to a page boundary.
/// - The length is not aligned to a page boundary.
/// - The target address and length refer to a memory region that overlaps completely or partially
///   with the higher half.
/// - Some set flags are not known to the kernel.
///
/// [`SysResult::INVALID_PROCESS_ID`] is returned if the provided `process_id` is not a valid
/// process id.
#[cfg(feature = "userland")]
#[inline(always)]
pub fn map_memory(
    process_id: Option<ProcessId>,
    virtual_address: usize,
    length: usize,
    flags: MapFlags,
) -> SysResult {
    SysResult(raw::syscall4(
        Syscall::MapMemory as usize,
        process_id.map_or(0, ProcessId::get),
        virtual_address,
        length,
        flags.bits(),
    ))
}

/// Unmaps a bunch of pages from the virtual address space of the specified process.
///
/// If any of the pages specified in the range are not mapped, the system call will ignore those
/// pages and only unmap the one that are mapped.
///
/// # Arguments
///
/// - `process_id` is the ID of the process to unmap the memory from. 0 indicates the current
///   process.
///
/// - `virtual_address` is the virtual address to unmap the memory from. This must be aligned to a
///   page boundary.
///
/// - `length` is the length of the memory region to unmap. This must be aligned to a page
///   boundary.
///
/// # Notes
///
/// Attempting to unmap memory pages that are not currently mapped in the target process's virtual
/// address space has unspecified behavior. It may or may not produce an error, and the pages of the
/// range that are actually mapped may or may not be unmapped.
///
/// # Returns
///
/// On success, this function returns 0.
///
/// # Errors
///
/// [`SysResult::INVALID_VALUE`] is returned if:
///
/// - The target address is not aligned to a page boundary.
/// - The length is not aligned to a page boundary.
///
/// [`SysResult::INVALID_PROCESS_ID`] is returned if the provided `process_id` is not a valid
/// process id.
#[cfg(feature = "userland")]
#[inline(always)]
pub fn unmap_memory(
    process_id: Option<ProcessId>,
    virtual_address: usize,
    length: usize,
) -> SysResult {
    SysResult(raw::syscall3(
        Syscall::UnmapMemory as usize,
        process_id.map_or(0, ProcessId::get),
        virtual_address,
        length,
    ))
}

/// Acquires a framebuffer for the provided process.
///
/// # Arguments
///
/// - `process_id` is the ID of the process to acquire the framebuffer for. 0 indicates the current
///  process.
///
/// - `index` is the index of the framebuffer to acquire.
///
/// - `at` is the virtual address at which the framebuffer's memory should be mapped. This must be
/// aligned to a page boundary. If the provided memory region is already mapped, the old mappings
/// will be overwritten.
///
/// # Returns
///
/// On success, this function returns 0.
///
/// # Errors
///
/// [`SysResult::INVALID_VALUE`] is returned if:
///
/// - The provided index does not refer to a valid framebuffer.
/// - The provided virtual address is not aligned to a page boundary.
///
/// [`SysResult::INVALID_PROCESS_ID`] is returned if the provided `process_id` is not a valid
/// process id.
///
/// [`SysResult::CONFICT`] is returned if the requested framebuffer is already acquired by a
/// process.
#[inline(always)]
#[cfg(feature = "userland")]
pub fn acquire_framebuffer(process_id: Option<ProcessId>, index: usize, at: *mut u8) -> SysResult {
    SysResult(raw::syscall3(
        Syscall::AcquireFramebuffer as usize,
        process_id.map_or(0, ProcessId::get),
        index,
        at as usize,
    ))
}

/// Releases a framebuffer from the provided process and unmaps its from memory.
///
/// # Arguments
///
/// - `process_id` is the ID of the process to release the framebuffer from. 0 indicates the current
/// process.
///
/// - `index` is the index of the framebuffer to release.
///
/// # Returns
///
/// On success, this function returns 0.
///
/// # Errors
///
/// [`SysResult::INVALID_PROCESS_ID`] is returned if the provided `process_id` is not a valid
/// process id.
///
/// [`SysResult::INVALID_VALUE`] is returned if the provided index does not refer to a valid
/// framebuffer.
///
/// [`SysResult::CONFICT`] is returned if the requested framebuffer is not acquired by the
/// target process.
#[inline(always)]
#[cfg(feature = "userland")]
pub fn release_framebuffer(process_id: Option<ProcessId>, index: usize) -> SysResult {
    SysResult(raw::syscall2(
        Syscall::ReleaseFramebuffer as usize,
        process_id.map_or(0, ProcessId::get),
        index,
    ))
}
