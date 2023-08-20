//! This module contains the implementation of all system calls!

use core::sync::atomic::Ordering::*;

use fabric_sys::x86_64::public::PublicData;
use fabric_sys::x86_64::MapFlags;
use fabric_sys::SysResult;

use crate::x86_64::mem::{MemoryTrackerTok, HHDM_OFFSET, PAGE_SIZE, USER_TOP};
use crate::x86_64::process::CURRENT_PROCESS;
use crate::x86_64::raw::{self, PageFlags};

/// Handles the `terminate` system call.
pub extern "C" fn terminate(
    process_id: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    todo!("terminate({})", process_id);
}

/// Handles the `map_memory` system call.
pub extern "C" fn map_memory(
    process_id: usize,
    mut virtual_address: usize,
    mut length: usize,
    flags: usize,
    _: usize,
    _: usize,
) -> SysResult {
    //
    // Validate the arguments.
    //
    let process = if process_id != 0 {
        return SysResult::INVALID_PROCESS_ID;
    } else {
        unsafe { &mut CURRENT_PROCESS }
    };

    if virtual_address % PAGE_SIZE != 0 || length % PAGE_SIZE != 0 {
        return SysResult::INVALID_VALUE;
    }

    if virtual_address.saturating_add(length) > USER_TOP {
        return SysResult::INVALID_VALUE;
    }

    let Some(flags) = MapFlags::from_bits(flags) else { return SysResult::INVALID_VALUE };

    //
    // Convert the flags into the format used by the CPU.
    //
    let mut page_flags = raw::PageFlags::USER;

    if flags.contains(MapFlags::WRITABLE) {
        page_flags.insert(raw::PageFlags::WRITABLE);
    }

    if !flags.contains(MapFlags::EXECUTABLE) {
        page_flags.insert(raw::PageFlags::NO_EXECUTE);
    }

    // SAFETY:
    //  The memory tracker is initialized before system calls are enabled.
    let memory_tracker = unsafe { MemoryTrackerTok::unchecked() };
    let mut memory_tracker = memory_tracker.lock();

    //
    // Allocate memory until we have mapped the entire requested region.
    //
    while length != 0 {
        let Ok(phys) =  memory_tracker.allocate() else { return SysResult::OUT_OF_MEMORY };

        // Map the page.
        if unsafe {
            crate::x86_64::cpu::paging::map_4kib(
                &mut *((process.address_space + HHDM_OFFSET) as *mut _),
                HHDM_OFFSET,
                &mut || memory_tracker.allocate(),
                virtual_address,
                phys,
                page_flags,
            )
            .is_err()
        } {
            return SysResult::OUT_OF_MEMORY;
        }

        crate::x86_64::instr::invlpg(virtual_address);

        length -= PAGE_SIZE;
        virtual_address += PAGE_SIZE;
    }

    SysResult::success(0)
}

/// Handles the `unmap_memory` system call.
pub extern "C" fn unmap_memory(
    process_id: usize,
    mut virtual_address: usize,
    mut length: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    //
    // Validate the arguments.
    //
    let process = if process_id != 0 {
        return SysResult::INVALID_PROCESS_ID;
    } else {
        unsafe { &mut CURRENT_PROCESS }
    };

    if virtual_address % PAGE_SIZE != 0 || length % PAGE_SIZE != 0 {
        return SysResult::INVALID_VALUE;
    }

    if virtual_address.saturating_add(length) > USER_TOP {
        return SysResult::INVALID_VALUE;
    }

    //
    // Mark the memory as free.
    //

    // SAFETY:
    //  The memory tracker is known to be initialized before system calls are enabled.
    let memory_tracker = unsafe { MemoryTrackerTok::unchecked() };

    while length != 0 {
        unsafe {
            // The documentation (that we wrote) indicates that attempting to unmap a page that's
            // not currently mapped has unspecified behavior. In our case, we'll just ignore the
            // error and only mark the page as free if it was actually previously mapped.
            let was_used = crate::x86_64::cpu::paging::unmap_4kib(
                &mut *((process.address_space + HHDM_OFFSET) as *mut _),
                HHDM_OFFSET,
                virtual_address,
            )
            .is_ok();

            if was_used {
                memory_tracker.lock().mark_as_unused(virtual_address);
            }
        }

        crate::x86_64::instr::invlpg(virtual_address);

        virtual_address += PAGE_SIZE;
        length -= PAGE_SIZE;
    }

    SysResult::success(0)
}

pub extern "C" fn acquire_framebuffer(
    process_id: usize,
    index: usize,
    mut at: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    let process = if process_id != 0 {
        return SysResult::INVALID_PROCESS_ID;
    } else {
        unsafe { &mut CURRENT_PROCESS }
    };

    let public = unsafe { &*(crate::x86_64::public_data_address() as *mut PublicData) };

    let Some(framebuffer) = public.framebuffers().get(index) else {
        return SysResult::INVALID_VALUE;
    };

    if framebuffer
        .owned_by
        .compare_exchange(0, process_id, AcqRel, Relaxed)
        .is_err()
    {
        return SysResult::CONFLICT;
    }

    let memory_tracker = unsafe { MemoryTrackerTok::unchecked() };
    let mut memory_tracker = memory_tracker.lock();

    // Map the framebuffer into the process's address space at the address they requested.
    let mut size = framebuffer.size_in_bytes();
    let mut addr = framebuffer.physical_address;
    while size != 0 {
        // Map the page.
        if unsafe {
            crate::x86_64::cpu::paging::map_4kib(
                &mut *((process.address_space + HHDM_OFFSET) as *mut _),
                HHDM_OFFSET,
                &mut || memory_tracker.allocate(),
                at,
                addr,
                PageFlags::USER | PageFlags::WRITABLE | PageFlags::NO_EXECUTE,
            )
            .is_err()
        } {
            return SysResult::OUT_OF_MEMORY;
        }

        crate::x86_64::instr::invlpg(at);

        size -= PAGE_SIZE;
        at += PAGE_SIZE;
        addr += PAGE_SIZE;
    }

    SysResult::success(0)
}

pub extern "C" fn release_framebuffer(
    process_id: usize,
    index: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
) -> SysResult {
    let _process = if process_id != 0 {
        return SysResult::INVALID_PROCESS_ID;
    } else {
        unsafe { &mut CURRENT_PROCESS }
    };

    let public = unsafe { &*(crate::x86_64::public_data_address() as *mut PublicData) };

    let Some(framebuffer) = public.framebuffers().get(index) else {
        return SysResult::INVALID_VALUE;
    };

    // TODO: the memory should be unmapped.

    if framebuffer
        .owned_by
        .compare_exchange(process_id, 0, AcqRel, Relaxed)
        .is_err()
    {
        return SysResult::CONFLICT;
    }

    SysResult::success(0)
}
