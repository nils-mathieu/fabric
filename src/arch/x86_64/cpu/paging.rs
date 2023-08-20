use core::mem::MaybeUninit;

use crate::log;

use crate::x86_64::mem::{BootAllocator, OutOfMemory, HHDM_OFFSET, PAGE_SIZE};
use crate::x86_64::raw::PageFlags;

const ONE_GIB: usize = 1024 * 1024 * 1024;
const TWO_MIB: usize = 1024 * 1024 * 2;
const FOUR_KIB: usize = 1024 * 4;

/// Fuses the provided flags.
///
/// This function preserves the `ACCESSED` flag and the user-defined flags of `a`.
fn fuse_flags(a: u64, b: u64) -> u64 {
    debug_assert!(
        b & PRESERVED_BITS == 0,
        "potential loss of data ({:#x})",
        b & PRESERVED_BITS
    );
    debug_assert!(a & PageFlags::HUGE.bits() == 0);
    debug_assert!(b & PageFlags::HUGE.bits() == 0);

    const OR_FLAGS: PageFlags = PageFlags::DIRTY
        .union(PageFlags::PRESENT)
        .union(PageFlags::WRITABLE)
        .union(PageFlags::USER);
    const AND_FLAGS: PageFlags = PageFlags::DISABLE_CACHE
        .union(PageFlags::GLOBAL)
        .union(PageFlags::NO_EXECUTE)
        .union(PageFlags::WRITE_THROUGH);
    const PRESERVED_BITS: u64 = 0x3FF0_FFFF_FFFF_FE00 | PageFlags::ACCESSED.bits();

    let a_and = a & AND_FLAGS.bits();
    let b_and = b & AND_FLAGS.bits();
    let a_or = a & OR_FLAGS.bits();
    let b_or = b & OR_FLAGS.bits();
    let a_kept = a & PRESERVED_BITS;

    // The flags that are part of the `OR_FLAGS` are simply fused together, while the flags that
    // are part of the `AND_FLAGS` should only remain active if they are active in both flags.
    a_kept | a_or | b_or | (a_and & b_and)
}

/// A page table.
#[repr(align(4096))]
pub struct PageTable(pub [u64; 512]);

impl PageTable {
    /// Returns a mutable reference to the page table entry at the given index.
    ///
    /// # Safety
    ///
    /// `index` must be less than 512.
    #[inline(always)]
    unsafe fn entry_mut(&mut self, index: usize) -> &mut u64 {
        debug_assert!(index < 512);
        unsafe { self.0.get_unchecked_mut(index) }
    }

    /// Returns a mutable reference to the page table entry at the given index.
    ///
    /// If the provided entry is already mapped, it is simply returned as a pointer to a
    /// [`PageTable`]. Otherwise, a new page table is allocated and mapped.
    ///
    /// # Safety
    ///
    /// `index` must be less than 512.
    ///
    /// If it is already mapped, the entry must be a valid page table.
    ///
    /// `direct_map` can be used to compute the virtual address of a given physical address.
    unsafe fn directory_entry_mut(
        &mut self,
        direct_map: usize,
        alloc_page: &mut dyn FnMut() -> Result<usize, OutOfMemory>,
        index: usize,
        parent_flags: PageFlags,
    ) -> Result<&mut PageTable, OutOfMemory> {
        let entry = unsafe { self.entry_mut(index) };

        let page;
        if *entry == 0 {
            // The directory entry is not present. We have to allocate a page table for this.
            page = alloc_page()?;

            unsafe { core::ptr::write_bytes((page + direct_map) as *mut PageTable, 0x00, 1) };

            *entry = page as u64 | (PageFlags::PRESENT | parent_flags).bits();
        } else {
            // The directory is already present. We need to extract the address.
            *entry = fuse_flags(*entry, parent_flags.bits());
            page = (*entry & 0x0fffffff_fffff000) as usize;
        }

        debug_assert!((page + direct_map) % PAGE_SIZE == 0);
        Ok(unsafe { &mut *((page + direct_map) as *mut PageTable) })
    }

    /// Tries to return a reference to the page table entry at the given index.
    ///
    /// # Safety
    ///
    /// `index` must be less than 512.
    unsafe fn try_directory_entry_mut(
        &mut self,
        direct_map: usize,
        index: usize,
    ) -> Option<&mut PageTable> {
        let entry = unsafe { *self.entry_mut(index) };

        if entry & PageFlags::PRESENT.bits() == 0 {
            return None;
        }

        if entry & PageFlags::HUGE.bits() != 0 {
            return None;
        }

        let phys_addr = entry & 0x0FFFFFFF_FFFFF000;
        let virt_addr = phys_addr as usize + direct_map;

        Some(unsafe { &mut *(virt_addr as *mut PageTable) })
    }
}

/// Maps a 4k page.
///
/// This function assumes that `phys` and `virt` are aligned to 4 KiB.
///
/// # Safety
///
/// `direct_map` can be used to compute the virtual address of a given physical address.
pub unsafe fn map_4kib(
    l4: &mut PageTable,
    direct_map: usize,
    alloc_page: &mut dyn FnMut() -> Result<usize, OutOfMemory>,
    virt: usize,
    phys: usize,
    flags: PageFlags,
) -> Result<(), OutOfMemory> {
    debug_assert!(phys % FOUR_KIB == 0);
    debug_assert!(virt % FOUR_KIB == 0);

    let l4_idx = (virt >> 39) & 0o777;
    let l3_idx = (virt >> 30) & 0o777;
    let l2_idx = (virt >> 21) & 0o777;
    let l1_idx = (virt >> 12) & 0o777;

    let l3 = unsafe { l4.directory_entry_mut(direct_map, alloc_page, l4_idx, flags)? };
    let l2 = unsafe { l3.directory_entry_mut(direct_map, alloc_page, l3_idx, flags)? };
    let l1 = unsafe { l2.directory_entry_mut(direct_map, alloc_page, l2_idx, flags)? };

    let entry = unsafe { l1.entry_mut(l1_idx) };
    *entry = phys as u64 | (PageFlags::PRESENT | flags).bits();

    Ok(())
}

/// Maps a 2 MiB page.
///
/// This function assumes that `phys` and `virt` are aligned to 2 MiB.
///
/// # Safety
///
/// `direct_map` can be used to compute the virtual address of a given physical address.
pub unsafe fn map_2mib(
    l4: &mut PageTable,
    direct_map: usize,
    alloc_page: &mut dyn FnMut() -> Result<usize, OutOfMemory>,
    virt: usize,
    phys: usize,
    flags: PageFlags,
) -> Result<(), OutOfMemory> {
    debug_assert!(phys % TWO_MIB == 0);
    debug_assert!(virt % TWO_MIB == 0);

    let l4_idx = (virt >> 39) & 0o777;
    let l3_idx = (virt >> 30) & 0o777;
    let l2_idx = (virt >> 21) & 0o777;

    let l3 = unsafe { l4.directory_entry_mut(direct_map, alloc_page, l4_idx, flags)? };
    let l2 = unsafe { l3.directory_entry_mut(direct_map, alloc_page, l3_idx, flags)? };

    let entry = unsafe { l2.entry_mut(l2_idx) };
    *entry = phys as u64 | (PageFlags::PRESENT | PageFlags::HUGE | flags).bits();

    Ok(())
}

/// Maps a 1 GiB page.
///
/// This function assumes that `phys` and `virt` are aligned to 1 GiB.
///
/// # Safety
///
/// `direct_map` can be used to compute the virtual address of a given physical address.
pub unsafe fn map_1gib(
    l4: &mut PageTable,
    direct_map: usize,
    alloc_page: &mut dyn FnMut() -> Result<usize, OutOfMemory>,
    virt: usize,
    phys: usize,
    flags: PageFlags,
) -> Result<(), OutOfMemory> {
    debug_assert!(phys % ONE_GIB == 0);
    debug_assert!(virt % ONE_GIB == 0);

    let l4_idx = (virt >> 39) & 0o777;
    let l3_idx = (virt >> 30) & 0o777;

    let l3 = unsafe { l4.directory_entry_mut(direct_map, alloc_page, l4_idx, flags)? };

    let entry = unsafe { l3.entry_mut(l3_idx) };
    *entry = phys as u64 | (PageFlags::PRESENT | PageFlags::HUGE | flags).bits();

    Ok(())
}

/// Unmaps a page of size 4 KiB.
///
/// # Returns
///
/// If the page was not mapped, `Err(())` is returned. Otherwise, `Ok(())` is returned.
///
/// # Safety
///
/// `direct_map` can be used to compute the virtual address of a given physical address.
pub unsafe fn unmap_4kib(l4: &mut PageTable, direct_map: usize, virt: usize) -> Result<(), ()> {
    debug_assert!(virt % FOUR_KIB == 0);

    let l4_idx = (virt >> 39) & 0o777;
    let l3_idx = (virt >> 30) & 0o777;
    let l2_idx = (virt >> 21) & 0o777;
    let l1_idx = (virt >> 12) & 0o777;

    unsafe {
        let l3 = l4.try_directory_entry_mut(direct_map, l4_idx).ok_or(())?;
        let l2 = l3.try_directory_entry_mut(direct_map, l3_idx).ok_or(())?;
        let l1 = l2.try_directory_entry_mut(direct_map, l2_idx).ok_or(())?;

        let entry = l1.entry_mut(l1_idx);
        if *entry & PageFlags::PRESENT.bits() == 0 {
            return Err(());
        }

        *entry = 0;
    }

    Ok(())
}

/// Creates a direct mapping for the given physical address.
///
/// Both `phys` and `virt` must be aligned to the page size. The size may or may not be aligned as
/// well.
///
/// # Safety
///
/// `direct_map` can be used to compute the virtual address of a given physical address.
#[allow(clippy::too_many_arguments)]
pub unsafe fn create_direct_map(
    l4: &mut PageTable,
    direct_map: usize,
    alloc_page: &mut dyn FnMut() -> Result<usize, OutOfMemory>,
    mut phys: usize,
    mut virt: usize,
    mut size: usize,
    flags: PageFlags,
) -> Result<(), OutOfMemory> {
    debug_assert!(phys % PAGE_SIZE == 0);
    debug_assert!(virt % PAGE_SIZE == 0);

    if size == 0 {
        return Ok(());
    }

    loop {
        if size >= ONE_GIB && phys % ONE_GIB == 0 && virt % ONE_GIB == 0 {
            unsafe { map_1gib(l4, direct_map, alloc_page, virt, phys, flags)? };

            size -= ONE_GIB;
            virt += ONE_GIB;
            phys += ONE_GIB;
        } else if size >= TWO_MIB && phys % TWO_MIB == 0 && virt % TWO_MIB == 0 {
            unsafe { map_2mib(l4, direct_map, alloc_page, virt, phys, flags)? };

            size -= TWO_MIB;
            virt += TWO_MIB;
            phys += TWO_MIB;
        } else {
            unsafe { map_4kib(l4, direct_map, alloc_page, virt, phys, flags)? };

            if size <= FOUR_KIB {
                break;
            }

            size -= FOUR_KIB;
            virt += FOUR_KIB;
            phys += FOUR_KIB;
        }
    }

    Ok(())
}

/// Initialize the page table to be used by the kernel.
///
/// This function will do two things:
///
/// 1. Create a direct mapping of the physical memory in the higher half (starting at
///    [`HHDM_OFFSET`]). The amount of memory actually mapped there is specified by the
///    `direct_map_size` argument.
///
/// 2. Map the kernel at the position specified by [`crate::x86_64::image_begin`]. It is exepcted
///    to be located at `kernel_start` in physical memory.
///
/// Note that of those two areas (the direct map and the kernel) overlap in any ways, the kernel
/// will take precedence and overwrite the direct map. This is checked in debug mode and will
/// panic if it happens.
///
/// # Arguments
///
/// This function assumes a direct mapping between physical and virtual memory, and will use
/// `direct_map` to compute the virtual address of a given physical address.
///
/// - `direct_map_size` is the amount of memory to be mapped in the higher half.
///
/// - `kernel_start` is the start address of the kernel in physical memory.
///
/// - `public_data_size` is the size of the public data section of the kernel.
///
/// - `public_data_phys` is the physical address of the public data section of the kernel.
///
/// # Safety
///
/// `direct_map` must be a valid offset between physical memory and a direct mapping.
///
/// # Returns
///
/// This function returns the physical address of the l4 page table that was created.
#[inline] // this function is only called once
pub unsafe fn create_kernel_address_space(
    direct_map: usize,
    boot_allocator: &mut BootAllocator,
    mut direct_map_size: usize,
    mut kernel_start: usize,
    public_data_phys: usize,
    public_data_size: usize,
) -> Result<usize, OutOfMemory> {
    log::trace!("Creating the kernel address space...");

    // If the kernel is not page aligned, we need to round down the start address.
    // Because that address is being rounded down, we need to add the difference to the size.
    let mut kernel_size = crate::x86_64::image_end() - crate::x86_64::image_begin();
    kernel_size += kernel_start & !0xFFF;
    kernel_start = crate::utility::align_page_down(kernel_start);

    // The direct map must be at least four gigabytes large as some I/O devices are mapped there.
    if direct_map_size < ONE_GIB * 4 {
        direct_map_size = ONE_GIB * 4;
    }

    debug_assert!(
        crate::x86_64::image_begin() >= HHDM_OFFSET + direct_map_size,
        "the kernel and the higher half direct map overlap"
    );

    let l4 = boot_allocator.allocate(PAGE_SIZE, PAGE_SIZE)?;
    unsafe { core::ptr::write_bytes((l4 + direct_map) as *mut PageTable, 0x00, 1) };

    let mut alloc_page = || boot_allocator.allocate(PAGE_SIZE, PAGE_SIZE);

    unsafe {
        // Create a direct mapping between physical memory and the higher half.
        create_direct_map(
            &mut *((l4 + direct_map) as *mut PageTable),
            direct_map,
            &mut alloc_page,
            0,
            HHDM_OFFSET,
            direct_map_size,
            PageFlags::WRITABLE | PageFlags::GLOBAL,
        )?;

        // Map the kernel.
        create_direct_map(
            &mut *((l4 + direct_map) as *mut PageTable),
            direct_map,
            &mut alloc_page,
            kernel_start,
            crate::x86_64::image_begin(),
            kernel_size,
            PageFlags::WRITABLE | PageFlags::GLOBAL,
        )?;

        // Map the public data.
        log::trace!(
            "The public data is mapped at address {:#x}.",
            crate::x86_64::public_data_address()
        );
        create_direct_map(
            &mut *((l4 + direct_map) as *mut PageTable),
            direct_map,
            &mut alloc_page,
            public_data_phys,
            crate::x86_64::public_data_address(),
            public_data_size,
            PageFlags::WRITABLE | PageFlags::GLOBAL | PageFlags::USER,
        )?;
    }

    Ok(l4)
}

/// The physical address of the L4 page table that contains the kernel address space.
static mut L4_TABLE: MaybeUninit<usize> = MaybeUninit::uninit();

/// A "token" type that proves the global address space has been initialized.
#[derive(Clone, Copy)]
pub struct UpperHalfAddressSpaceTok(());

impl UpperHalfAddressSpaceTok {
    /// Returns the [`UpperHalfAddressSpaceTok`] token.
    ///
    /// # Safety
    ///
    /// The global address space must've been initialized before calling the function.
    #[inline(always)]
    pub const unsafe fn unchecked() -> Self {
        Self(())
    }

    /// Creates a new token.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it can only be called once.
    #[inline(always)]
    pub unsafe fn init(l4_table: usize) -> Self {
        unsafe {
            L4_TABLE = MaybeUninit::new(l4_table);
            Self::unchecked()
        }
    }

    /// Returns the physical address of the L4 page table that contains the kernel address space.
    #[inline(always)]
    pub fn get(self) -> usize {
        unsafe { L4_TABLE.assume_init() }
    }
}
