use super::OutOfMemory;

/// The "boot allocator" is responsible for allocating pages during the boot process. Pages
/// allocated by this provider cannot be trivially deallocated (this is a bump allocator).
///
/// This is useful to allocate the initial structures (stacks, page tables) required globally by
/// the operating system that won't ever be deallocated during the lifetime of the kernel.
///
/// When a proper allocator is set up, it should take in account the memory allocated by this
/// provider to avoid overwriting the data.
pub struct BootAllocator {
    /// The base address of the available block.
    start: usize,
    /// The first address that is not available.
    stop: usize,
}

impl BootAllocator {
    /// Creates a new page provider.
    ///
    /// # Arguments
    ///
    /// * `base`: The base address of the first page that will be allocated.
    /// * `length`: The number of bytes that are available for allocation, starting at `base`.
    ///
    /// This function will automatically align `base` and `length` to the page size. If an invalid
    /// value is passed, the function returns an allocator that cannot allocate anything (i.e. of
    /// length 0).
    pub fn new(base: usize, length: usize) -> Self {
        Self {
            start: base,
            stop: base + length,
        }
    }

    /// Returns the number of bytes that are still available for allocation.
    #[inline(always)]
    pub fn remaining_length(&self) -> usize {
        self.stop - self.start
    }

    /// Returns the physical address of the next page that will be allocated.
    #[inline(always)]
    pub fn peek(&self) -> usize {
        self.start
    }

    /// Allocates zero or more bytes of physical memory.
    ///
    /// # Arguments
    ///
    /// * `size`: The number of bytes to allocate.
    ///
    /// * `align`: The alignment of the allocation. Must be a power of two.
    ///
    /// # Returns
    ///
    /// This function returns [`Err(_)`] if the allocation failed (i.e. there is not enough memory
    /// in the managed block). Otherwise, it returns the address of the first allocated page in
    /// the higher half direct map.
    pub fn allocate(&mut self, size: usize, align: usize) -> Result<usize, OutOfMemory> {
        debug_assert!(align.is_power_of_two());

        let align_mask = align - 1;
        let ret = (self.start + align_mask) & !align_mask;

        if ret + size - self.start > self.remaining_length() {
            return Err(OutOfMemory);
        }

        self.start = ret + size;

        Ok(ret)
    }
}
