//! This module contains the entry point of the kernel, when loaded by a Limine-complient
//! bootloader.
//!
//! See the [*protocol*][1] for a detailed description of the interface between the kernel and the
//! bootloader.
//!
//! [1]: https://github.com/limine-bootloader/limine/blob/v4.x-branch/PROTOCOL.md

use core::arch::asm;
use core::sync::atomic::AtomicUsize;

use fabric_sys::x86_64::public::{ColorMode, Framebuffer, PublicData};
use fabric_sys::InitHeader;

use crate::log;
use crate::x86_64::mem::{
    BootAllocator, MemoryTrackerTok, HHDM_OFFSET, MAX_PHYSICAL_MEMORY, PAGE_SIZE,
};
use crate::x86_64::public::PublicDataLayout;

use super::cpu::paging::UpperHalfAddressSpaceTok;

mod raw;
mod req;

/// The maximum number of contiguous memory segments that the bootloader can provide.
///
/// 16 is very pessimistic. There will usually be at most 4 to 6 segments.
const MAX_SEGMENTS: usize = 16;

/// Represents a physical memory segment.
#[derive(Clone, Copy)]
struct MemorySegment {
    /// The base physical address of the segment.
    ///
    /// This should always be a multiple of the page size.
    base: usize,
    /// The size of the segment, in bytes.
    ///
    /// This should always be a multiple of the page size.
    length: usize,
}

/// The structure that is passed from the bootloader stack to the new stack allocated by the
/// boot allocator.
struct Transfer {
    boot_allocator: BootAllocator,
    segments: [MemorySegment; MAX_SEGMENTS],
    boot_allocator_start_address: usize,
    fabric_init_start_address: usize,
    fabric_init_size: usize,
    upper_half_address_space: UpperHalfAddressSpaceTok,
}

/// The entry point of the kernel, when loaded by a Limine-complient bootloader.
///
/// This function is selected by Limine because it is specified in an "entry point" request embeded
/// in the kernel ELF image.
///
/// # Safety
///
/// This function must be called only once. It expects the machine to be in the state specified by
/// the Limine protocol.
///
/// This function assumes that the Limine bootloader complies with the protocol specification. The
/// function will attempt to sanitize the responses to the requests made to the bootloader, but
/// some things, such as null-terminated strings, cannot be checked and will cause unpredictable
/// behavior.
unsafe extern "C" fn entry_point() -> ! {
    // Those variables are initialized within the scope defined bellow.
    // The bootloader reclaimable memory will become invalid at the end of that scope and those
    // variables are extracted from it.
    let mut boot_allocator: BootAllocator;
    let mut segments = [MemorySegment { base: 0, length: 0 }; MAX_SEGMENTS];

    // Response pointers provided by the bootloader may only be used within this scope. After that
    // point, a new page table will be loaded and the higher half direct map set up by the
    // bootloader will be invalidated.

    // This "token" must be dropped *before* the main memory allocator is initialized. After
    // that, the bootloader reclaimable memory will be reclaimed and might be overwritten. This
    // means that any pointer provided by the bootloader will become invalid.
    let limine = req::LimineTok::unchecked();

    // Initialize the logger.
    let serial = unsafe { crate::x86_64::serial::SerialTok::init() };
    crate::log::set_global_log_fn(serial.log_fn());
    log::trace!("Logger initialized.");

    req::validate_entry_point(limine);
    req::log_bootloader_info(limine);

    let current_hhdm = req::hhdm_offset(limine);

    let fabric_init = req::fabric_init(limine);

    // Find the physical address of the `fabric_init` module.
    // Note that we can't simply use the virtual address provided by the bootloader because it
    // points to the higher half direct map that we will replace soon with our own. Instead, we
    // save the physical address of the module which won't depend on the current address space.
    let fabric_init_start_address = fabric_init.as_ptr() as usize - current_hhdm;
    let fabric_init_size = fabric_init.len();

    log::trace!(
        "Found `fabric_init` module of size {}.",
        crate::utility::HumanByteCount(fabric_init.len() as u64)
    );

    let memmap = req::memory_map(limine);

    // Compute the contiguous memory segments provided by the bootloader.
    let mut largest_segment: Option<usize> = None;
    let mut segment_count: usize = 0;
    for segment in memmap.iter() {
        if segment.type_ != raw::MEMMAP_USABLE
            && segment.type_ != raw::MEMMAP_ACPI_RECLAIMABLE
            && segment.type_ != raw::MEMMAP_BOOTLOADER_RECLAIMABLE
        {
            continue;
        }

        if segment_count >= MAX_SEGMENTS {
            log::warn!("Too many memory segments provided by the bootloader.");
            log::warn!("Only the first {} will be used.", MAX_SEGMENTS);

            let amount = segments.iter().map(|s| s.length).sum::<usize>();
            log::warn!(
                "Usable memory: {}.",
                crate::utility::HumanByteCount(amount as u64)
            );

            break;
        }

        // We're looking for the largest segment that is usable (not reserved or currently
        // used by the bootloader).
        if segment.type_ == raw::MEMMAP_USABLE {
            match &mut largest_segment {
                Some(idx) => {
                    let cur = unsafe { segments.get_unchecked_mut(*idx) };
                    if cur.length < segment.length as usize {
                        *idx = segment_count;
                    }
                }
                _ => {
                    largest_segment = Some(segment_count);
                }
            }
        }

        if let Some(prev_idx) = segment_count.checked_sub(1) {
            // SAFETY:
            //  `segment_count` is always bellow `MAX_SEGMENTS`, so `prev_idx` is always valid.
            let prev = unsafe { segments.get_unchecked_mut(prev_idx) };

            if prev.base + prev.length == segment.base as usize {
                // Extend the previous segment.
                prev.length += segment.length as usize;
                continue;
            }
        }

        // Either there was no previous segment, or segments could not be merged.
        // Create a new segment.
        let seg = unsafe { segments.get_unchecked_mut(segment_count) };

        seg.base = segment.base as usize;
        seg.length = segment.length as usize;

        segment_count += 1;
    }

    // Re-align the segments to page boundaries.
    //
    // This is normally not necessary for the Limine bootloader (usable segments are guarenteed
    // to be page aligned). However, we don't want to crash if a bootloader does not comply
    // perfectly with the standard.
    for segment in &mut segments {
        // This aligns the base up to the next page boundary, and the length down.
        let offset = segment.base % PAGE_SIZE;
        if offset != 0 {
            segment.base += PAGE_SIZE - offset;
            segment.length -= PAGE_SIZE - offset;
        }
        segment.length -= segment.length % PAGE_SIZE;
    }

    let largest_segment = largest_segment.unwrap_or_else(|| oom());
    let largest_segment = unsafe { *segments.get_unchecked(largest_segment) };

    let boot_allocator_start_address = largest_segment.base;
    boot_allocator = BootAllocator::new(boot_allocator_start_address, largest_segment.length);

    log::trace!(
        "Boot allocator initialized with a contiguous block of {}.",
        crate::utility::HumanByteCount(boot_allocator.remaining_length() as u64),
    );

    // Find the upper bound of the direct map.
    let mut direct_map_size = memmap
        .iter()
        .filter(|e| {
            e.type_ == raw::MEMMAP_USABLE
                || e.type_ == raw::MEMMAP_BOOTLOADER_RECLAIMABLE
                || e.type_ == raw::MEMMAP_ACPI_RECLAIMABLE
        })
        .map(|e| (e.base + e.length) as usize)
        .max()
        .unwrap_or(0);

    if direct_map_size > MAX_PHYSICAL_MEMORY {
        log::warn!(
            "Detected {} of physical memory.",
            crate::utility::HumanByteCount(direct_map_size as u64)
        );
        log::warn!(
            "Only up to {} of physical memory are supported.",
            crate::utility::HumanByteCount(MAX_PHYSICAL_MEMORY as u64),
        );
        log::warn!("");
        log::warn!("This is due to the laziness of the Fabric developers.");
        log::warn!("If this is a problem for you, please open an issue at:");
        log::warn!("");
        log::warn!("    https://github.com/nils-mathieu/fabric/issues/new");

        direct_map_size = MAX_PHYSICAL_MEMORY;
    } else {
        log::info!(
            "Available physical memory: {}.",
            crate::utility::HumanByteCount(direct_map_size as u64)
        );
    }

    let framebuffers = req::framebuffers(limine);

    // Print a warning if a framebuffer is not supported.
    let mut first_unsupported_framebuffer = true;
    let mut supported_framebuffer_count = 0;
    for framebuffer in framebuffers {
        if !is_framebuffer_supported(framebuffer) {
            if first_unsupported_framebuffer {
                log::warn!("Found unsupported framebuffer(s):");
                first_unsupported_framebuffer = false;
            }

            log::warn!(
                "    {}x{}, {} bpp, mode {}",
                framebuffer.width,
                framebuffer.height,
                framebuffer.bpp,
                framebuffer.memory_model,
            );

            continue;
        }

        supported_framebuffer_count += 1;
    }

    log::trace!(
        "Found {} supported framebuffer(s).",
        supported_framebuffer_count
    );

    // Compute that amount of memory that we need to allocate for the public data area.
    let public_data_layout = PublicDataLayout::compute(supported_framebuffer_count);

    // Initialize the parts of the public data area that we can initialize now.
    // We kinda need to do this now because after we switch address spaces, we won't be able to
    // access the bootloader reclaimable memory anymore.
    let public_data_phys = boot_allocator
        .allocate(public_data_layout.size, PAGE_SIZE) // we need to be aligned to the page size to map the memory at a specific position later.
        .unwrap_or_else(|_| oom());

    // Initialize the root of the public data area. That's an instance of [`PublicData`] which
    // references the other parts of the public data area.
    //
    // Note that we're not initializing all parts of the public area just yet.
    unsafe {
        core::ptr::write(
            (public_data_phys + public_data_layout.root + current_hhdm) as *mut PublicData,
            PublicData {
                framebuffers: (crate::x86_64::public_data_address()
                    + public_data_layout.framebuffers)
                    as *const Framebuffer,
                framebuffer_count: framebuffers.len(),
            },
        );

        let mut cur =
            (public_data_phys + public_data_layout.framebuffers + current_hhdm) as *mut Framebuffer;
        for framebuffer in framebuffers {
            match try_convert_framebuffer(framebuffer, current_hhdm) {
                Some(framebuffer) => core::ptr::write(cur, framebuffer),
                None => continue,
            }
            cur = cur.add(1);
        }
    }

    let l4_table = unsafe {
        crate::x86_64::cpu::paging::create_kernel_address_space(
            current_hhdm,
            &mut boot_allocator,
            direct_map_size,
            req::kernel_physical_address(limine),
            public_data_phys,
            public_data_layout.size,
        )
        .unwrap_or_else(|_| oom())
    };

    let upper_half_address_space = unsafe { UpperHalfAddressSpaceTok::init(l4_table) };

    // We're currently running on the stack provided by the bootloader, which resides in bootloader
    // reclaimable memory. When a proper memory allocator is initialized, this memory will be
    // overwritten, so we need to switch to a stack that is guaranteed to be safe.
    let mut stack =
        unsafe { crate::x86_64::kernel_stack::init(&mut boot_allocator).unwrap_or_else(|_| oom()) };

    // Copy the Transfer structure to the new stack.
    stack -= core::mem::size_of::<Transfer>();
    unsafe {
        core::ptr::write(
            (stack + current_hhdm) as *mut Transfer,
            Transfer {
                segments,
                boot_allocator_start_address,
                boot_allocator,
                fabric_init_start_address,
                fabric_init_size,
                upper_half_address_space,
            },
        );
    }

    log::trace!("Switching to the created address space...");

    // /////////////////////////////////////////////////////////////////////////////////////////////
    // After this point, we can no longer use the memory provided by the bootloader. Pointers to
    // the bootloader reclaimable memory must be dropped before this point.
    // /////////////////////////////////////////////////////////////////////////////////////////////

    // Switch to the new stack and address space.
    //
    // Note that it's important that the stack and address space switch happen in the same `asm!`
    // block, as the Rust compiler is free to insert arbitrary instructions between two `asm!`
    // blocks. At this point, any interaction with the old stack would instantly trigger a page
    // fault (which isn't currently handled).
    //
    // We don't need to use `call`, as the `entry_point_follow` function will never return. The
    // current stack will be overwritten by the memory allocator anayway. We need to use a separate
    // function to ensure that Rust does not rely on the stack pointer being left unchanged.
    unsafe {
        asm!(
            r#"
            mov rsp, rdi
            mov rbp, rdi
            mov cr3, {l4_table}

            jmp {next}
            "#,
            l4_table = in(reg) l4_table,
            next = sym entry_point_follow,

            in("rdi") stack + HHDM_OFFSET,
            options(noreturn),
        );
    }
}

/// This function is called after the custom stack has been set up.
///
/// We need to use another function to make sure that Rust does not rely the stack pointer being
/// left unchanged.
///
/// # Safety
///
/// This function must be called only once.
///
/// `transfer` must be a valid pointer to a `Transfer` structure.
unsafe extern "C" fn entry_point_follow(transfer: *mut Transfer) -> ! {
    let Transfer {
        mut boot_allocator,
        segments,
        boot_allocator_start_address,
        fabric_init_start_address,
        fabric_init_size,
        upper_half_address_space,
    } = unsafe { transfer.read() };

    // SAFETY:
    //  - The function is only called once.
    //  - The kernel stack has been allocated (`gdt::init` can reference it in the TSS).
    unsafe {
        super::cpu::gdt::init(&mut boot_allocator).unwrap_or_else(|_| oom());
        super::cpu::idt::init();
    }

    log::trace!("Initializing the global memory tracker...");
    let nb_pages = segments
        .iter()
        .map(|e| e.base + e.length)
        .max()
        .unwrap_or_else(|| oom())
        / PAGE_SIZE;
    let mut memory_tracker = crate::x86_64::mem::MemoryTracker::new(nb_pages, &mut boot_allocator)
        .unwrap_or_else(|_| oom());

    // Push the free segments to the memory tracker.
    for segment in &segments {
        let mut segment = *segment;

        // The segment that contains the boot allocator needs to be split into two parts: the
        // part that contains the boot allocator, and the part that does not. The part that
        // has already been used must not be passed to the memory tracker.
        if segment.base == boot_allocator_start_address {
            let diff = boot_allocator.peek() - segment.base;

            debug_assert!(diff <= segment.length);

            segment.base += diff;
            segment.length -= diff;

            debug_assert!(segment.base == boot_allocator.peek());
        }

        let mut start = crate::utility::align_page_up(segment.base);
        let end = crate::utility::align_page_down(segment.base + segment.length);

        while start != end {
            // SAFETY:
            //  We allocate enough capacity of the memory tracker to hold as many segments as there
            //  is available pages. That's well enough to hold all segments.
            memory_tracker.mark_as_unused(start);
            start += PAGE_SIZE;
        }
    }

    let memory_tracker = unsafe { MemoryTrackerTok::init(memory_tracker) };

    unsafe { super::syscall::init() };

    log::trace!("Initializing the local APIC...");
    unsafe {
        super::cpu::apic::init_local_apic();
    }

    log::trace!("Now accepting interrupts!");
    super::instr::sti();

    // TODO:
    //  Initialize the scheduler.

    // TODO:
    //  Bootstrap other CPUs.

    log::trace!("Loading the `fabric_init` process...");

    // We converting numbers using the native endianness, as the kernel is not supposed to run
    // an init process that was compiled for a different endianness.
    // If a process is compiled for a different endianness, the magic number will be reversed and
    // we will be able to detect it.
    let fabric_init = unsafe {
        core::slice::from_raw_parts(
            (fabric_init_start_address + HHDM_OFFSET) as *const u8,
            fabric_init_size,
        )
    };

    if fabric_init.len() < core::mem::size_of::<InitHeader>() {
        log::error!("`fabric_init` is too small to hold the necessary header.");
        crate::die();
    }

    let header = unsafe { &*(fabric_init.as_ptr() as *const InitHeader) };

    if InitHeader::MAGIC != header.magic {
        log::error!("The `fabric_init` process does not have a valid header.");
        if header.magic == InitHeader::MAGIC.swap_bytes() {
            log::error!("It seems to have been compiled for a different endianness.");
        }
        crate::die();
    }

    // Perform some sanity checks on the entry point.
    //
    // This won't prevent all possible issues (far from it), but it should catch some errors early
    // on.
    if header.entry_point.is_null()
        || (header.entry_point as usize) < header.image_start as usize
        || (header.entry_point as usize) >= header.image_start as usize + fabric_init.len()
    {
        log::error!("The `fabric_init` process does not have a valid entry point.");
        crate::die();
    }

    // Map the init process in memory at the correct position.
    // We need to map the kernel in the upper half of the address space.
    // Luckily, we already have an address space correctly set up for this. We can simply copy
    // the upper half of the current address space.
    log::trace!("Creating the address space of the `fabric_init` process...");
    let new_l4_table;
    {
        use crate::x86_64::cpu::paging::{create_direct_map, PageTable};
        use crate::x86_64::raw;

        let l4_table = upper_half_address_space.get();

        let mut mem_tracker = memory_tracker.lock();
        new_l4_table = mem_tracker.allocate().unwrap_or_else(|_| oom());

        unsafe {
            core::ptr::copy_nonoverlapping(
                (l4_table + HHDM_OFFSET) as *mut PageTable,
                (new_l4_table + HHDM_OFFSET) as *mut PageTable,
                1,
            );

            create_direct_map(
                &mut *((new_l4_table + HHDM_OFFSET) as *mut PageTable),
                HHDM_OFFSET,
                &mut || mem_tracker.allocate(),
                fabric_init_start_address,
                header.image_start as usize,
                fabric_init_size,
                raw::PageFlags::WRITABLE | raw::PageFlags::USER,
            )
            .unwrap_or_else(|_| oom());
        }
    }

    // FIXME:
    //  This should actually use the scheduler.
    //  We don't have a scheduler yet.

    log::info!("Passing control to the `fabric_init` process...");

    unsafe {
        // We have to set the current process.
        crate::x86_64::process::CURRENT_PROCESS.address_space = new_l4_table;

        asm!(
            r#"
            mov cr3, {new_l4_table}
            sysretq
            "#,
            in("rcx") header.entry_point,
            in("r11") 0x202,
            new_l4_table = in(reg) new_l4_table,
            options(noreturn),
        );
    }
}

/// Dies with a message indicating that the kernel ran out of memory.
///
/// This is rarely an issue, but during the boot process, the kernel requires a certain amount of
/// memory to be available. If this requirement is not met, the kernel cannot continue.
fn oom() -> ! {
    log::error!("Not enough memory to initialize the kernel.");
    log::error!("Please download more RAM.");
    crate::die();
}

/// Returns whether the provided framebuffer is supported.
///
/// This function returns `true` if an only if `try_convert_framebuffer` would return `Some`.
fn is_framebuffer_supported(framebuffer: &raw::Framebuffer) -> bool {
    matches!(
        (framebuffer.memory_model, framebuffer.bpp),
        (raw::FRAMEBUFFER_RGB, 24) | (raw::FRAMEBUFFER_RGB, 32)
    )
}

/// Tries to convert the provided framebuffer into a `Framebuffer`.
fn try_convert_framebuffer(
    framebuffer: &raw::Framebuffer,
    hhdm_offset: usize,
) -> Option<Framebuffer> {
    let color_mode = match (framebuffer.memory_model, framebuffer.bpp) {
        (raw::FRAMEBUFFER_RGB, 24) => ColorMode::Rgb24,
        (raw::FRAMEBUFFER_RGB, 32) => ColorMode::Rgb32,
        _ => return None,
    };

    Some(Framebuffer {
        width: framebuffer.width as usize,
        height: framebuffer.height as usize,
        pitch: framebuffer.pitch as usize,
        color_mode,
        _reserved: [0; 7],
        physical_address: framebuffer.address as usize - hhdm_offset,
        owned_by: AtomicUsize::new(0),
    })
}
