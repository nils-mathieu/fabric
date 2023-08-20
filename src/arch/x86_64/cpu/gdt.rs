use core::mem::size_of;
use core::ptr::addr_of;

use crate::log;
use crate::x86_64::mem::{BootAllocator, OutOfMemory, HHDM_OFFSET, PAGE_SIZE};
use crate::x86_64::raw;
use crate::x86_64::raw::SegmentFlags;

pub const KERNEL_CODE_SELECTOR: u16 = 8;
pub const KERNEL_DATA_SELECTOR: u16 = 8 * 2;
pub const USER_DATA_SELECTOR: u16 = (8 * 3) | 0b11;
pub const USER_CODE_SELECTOR: u16 = (8 * 4) | 0b11;
pub const TSS_SELECTOR: u16 = 8 * 5;

/// The amount of the stack memory reserved for the double fault handler.
///
/// A separate stack is used for this CPU exception because it might be trigged by a stack overflow
/// within the kernel stack. In that case, the current kernel stack is unusable, and the double
/// fault will turn in a triple fault, causing the machine to reboot. Unlikely, but highly
/// undesirable.
const DOUBLE_FAULT_STACK_SIZE: usize = PAGE_SIZE * 4;

/// The index of the double fault stack in the *Interrupt Stack Table* of the TSS.
pub const DOUBLE_FAULT_STACK_INDEX: usize = 0;

/// The global descriptor table of the kernel that will be inserted into the bootstrap CPU.
///
/// This global is initialized by the [`init`] function, and must not be accessed before
/// initialization.
static mut GDT: [u64; 7] = [
    // Null Descriptor
    0,
    // Kernel Code Segment
    SegmentFlags::ACCESSED
        .union(SegmentFlags::PRESENT)
        .union(SegmentFlags::DATA)
        .union(SegmentFlags::EXECUTABLE)
        .union(SegmentFlags::READABLE)
        .union(SegmentFlags::LONG_MODE_CODE)
        .union(SegmentFlags::GRANULARITY)
        .union(SegmentFlags::LIMIT_MAX)
        .bits(),
    // Kernel Data Segment
    SegmentFlags::ACCESSED
        .union(SegmentFlags::PRESENT)
        .union(SegmentFlags::DATA)
        .union(SegmentFlags::WRITABLE)
        .union(SegmentFlags::SIZE_32BIT)
        .union(SegmentFlags::GRANULARITY)
        .union(SegmentFlags::LIMIT_MAX)
        .bits(),
    // User Data Segment
    SegmentFlags::ACCESSED
        .union(SegmentFlags::PRESENT)
        .union(SegmentFlags::DATA)
        .union(SegmentFlags::WRITABLE)
        .union(SegmentFlags::SIZE_32BIT)
        .union(SegmentFlags::USER)
        .union(SegmentFlags::GRANULARITY)
        .union(SegmentFlags::LIMIT_MAX)
        .bits(),
    // User Code Segment
    SegmentFlags::ACCESSED
        .union(SegmentFlags::PRESENT)
        .union(SegmentFlags::DATA)
        .union(SegmentFlags::EXECUTABLE)
        .union(SegmentFlags::READABLE)
        .union(SegmentFlags::LONG_MODE_CODE)
        .union(SegmentFlags::USER)
        .union(SegmentFlags::GRANULARITY)
        .union(SegmentFlags::LIMIT_MAX)
        .bits(),
    // Task State Segment
    0,
    0,
];

static mut GDT_DESC: raw::TableDesc = raw::TableDesc {
    base: unsafe { addr_of!(GDT) as *const () },
    limit: size_of::<[u64; 7]>() as u16 - 1,
};

/// The task state segment that will be inserted into the GDT of the bootstrap CPU.
static mut TSS: raw::TaskStateSegment = raw::TaskStateSegment {
    reserved0: 0,
    reserved1: 0,
    reserved2: 0,
    reserved3: 0,
    interrupt_stack_table: [0; 7],
    privilege_stack_table: [0; 3],
    iomap_base: 0,
};

/// Initializes a Global Descriptor Table for the kernel.
///
/// # Safety
///
/// This function must only be called once.
///
/// The kernel stack must've been initialized before calling this function.
#[inline] // only called once
pub unsafe fn init(boot_allocator: &mut BootAllocator) -> Result<(), OutOfMemory> {
    let double_fault_stack = boot_allocator.allocate(DOUBLE_FAULT_STACK_SIZE, 1)?
        + HHDM_OFFSET
        + DOUBLE_FAULT_STACK_SIZE;

    // SAFETY:
    //  Because this function can only be called once, we can safely assume that the GDT is not
    //  being used by any other CPU.
    //
    //  This function won't be called at a point where multiple CPUs are running concurrently
    //  anyway, as the kernel is not yet fully initialized at that point.
    unsafe {
        TSS.interrupt_stack_table[DOUBLE_FAULT_STACK_INDEX] = double_fault_stack as u64;
        TSS.privilege_stack_table[0] =
            (crate::x86_64::kernel_stack::KERNEL_STACK_TOP + HHDM_OFFSET) as u64;

        let tss_base = addr_of!(TSS) as u64;

        GDT[5] |= (size_of::<raw::TaskStateSegment>() as u64 - 1) & 0xFFFF; // this is always 0x67
        GDT[5] |= ((tss_base & 0xFFFFFF) << 16) | ((tss_base & 0xFF000000) << 32);
        GDT[5] |= (SegmentFlags::PRESENT | SegmentFlags::AVAILABLE_TSS).bits();
        GDT[6] |= tss_base >> 32;
    }

    log::trace!("Switching GDT...");

    unsafe {
        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &GDT_DESC,
            options(nostack, readonly, preserves_flags)
        );

        // Changing the code segment selector is always a bit tricky, as we can't directly use a
        // regular MOV instruction to do so. Instead, we use the RETFQ instruction to change the
        // code selector and jump to the next instruction.
        core::arch::asm!(
            r#"
            push {code_sel}
            lea {tmp}, [rip + 1f]
            push {tmp}
            retfq
        1:
            "#,
            code_sel = const KERNEL_CODE_SELECTOR as u64,
            tmp = lateout(reg) _,
            options(preserves_flags, nomem)
        );

        core::arch::asm!(
            r#"
            mov ds, {sel:x}
            mov es, {sel:x}
            mov fs, {sel:x}
            mov gs, {sel:x}
            mov ss, {sel:x}
            "#,
            sel = in(reg) KERNEL_DATA_SELECTOR,
            options(preserves_flags, nostack, nomem)
        );

        core::arch::asm!(
            "ltr {:x}",
            in(reg) TSS_SELECTOR,
            options(preserves_flags, nomem, nostack)
        );
    }

    Ok(())
}
