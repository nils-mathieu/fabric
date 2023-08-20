use crate::x86_64::mem::{BootAllocator, OutOfMemory, PAGE_SIZE};

/// The size of the kernel stack.
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 16;

/// The kernel stack top physical address.
pub static mut KERNEL_STACK_TOP: usize = 0;

/// Initializes the kernel stack.
///
/// # Safety
///
/// This function must be called once.
pub unsafe fn init(boot_allocator: &mut BootAllocator) -> Result<usize, OutOfMemory> {
    let base = boot_allocator.allocate(KERNEL_STACK_SIZE, 1)?;
    let top = base + KERNEL_STACK_SIZE;

    unsafe {
        KERNEL_STACK_TOP = top;
    }

    Ok(top)
}
