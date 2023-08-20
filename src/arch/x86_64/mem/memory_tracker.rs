use core::cell::UnsafeCell;
use core::mem::{align_of, size_of, MaybeUninit};
use core::ops::{Deref, DerefMut};

use super::{BootAllocator, OutOfMemory, HHDM_OFFSET, PAGE_SIZE};
use crate::utility::RawEpochMutex;

/// Stores metadata about a page.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrackedPage {}

/// Tracks the memory usage of the system.
///
/// This type keeps track of the state required to allocate new pages of physical memory and
/// manage the metadata associated with them.
#[repr(C)]
pub struct MemoryTracker {
    /// The number of pages that the tracker can manage.
    page_count: usize,
    /// The list of free pages.
    ///
    /// This list contains the indices within the `pages` array of all pages that are free. The
    /// physical address of the page can be retrieved by multiplying the index by the page size.
    free_pages: *mut usize,
    /// The total number of free pages referenced by `free_pages`.
    free_pages_len: usize,
}

unsafe impl Sync for MemoryTracker {}
unsafe impl Send for MemoryTracker {}

impl MemoryTracker {
    /// Creates a new empty [`MemoryTracker`]. By default, the memory tracker does not track
    /// any memore. Some must be pushed using [`MemoryTracker::push_segment_unchecked`].
    ///
    /// # Arguments
    ///
    /// `page_count` is the number of pages that the allocator should be able to manage. Note that
    /// more than `page_count` slots may be allocated to avoid wasting memory.
    pub fn new(page_count: usize, boot_allocator: &mut BootAllocator) -> Result<Self, OutOfMemory> {
        // We will allocate two arrayso: one for the page metadata, and the other for the list of
        // free pages.

        let free_pages = (boot_allocator
            .allocate(page_count * size_of::<usize>(), align_of::<usize>())?
            + HHDM_OFFSET) as *mut usize;

        Ok(Self {
            free_pages,
            free_pages_len: 0,
            page_count,
        })
    }

    /// Registers a new free page in the tracker.
    ///
    /// # Panics
    ///
    /// In debug builds, this function checks that its invariants are respected and panics if any
    /// of them are violated. It won't check whether the page is already registered as free,
    /// however, as this would be too expensive.
    ///
    /// # Safety
    ///
    /// - `page` must be a valid physical address (aligned to the page size).
    /// - `page` must not already be registered.
    /// - `page` must be within the range of pages managed by the tracker (i.e. less than the value
    ///   passed to [`MemoryTracker::new`]).
    #[inline]
    pub fn mark_as_unused(&mut self, page: usize) {
        #[cfg(debug_assertions)]
        {
            assert!(page % PAGE_SIZE == 0, "page is {:#x}", page);
            let index = page / PAGE_SIZE;
            assert!(index < self.page_count);
        }

        // SAFETY:
        //  The caller must ensure that the page is valid and not already registered as free.
        //  If the page is not already registered, `free_pages` is large enough to store it.
        unsafe {
            self.free_pages
                .add(self.free_pages_len)
                .write(page / PAGE_SIZE)
        };

        self.free_pages_len += 1;
    }

    /// Allocates a physical memory page.
    #[inline]
    pub fn allocate(&mut self) -> Result<usize, OutOfMemory> {
        if self.free_pages_len == 0 {
            return Err(OutOfMemory);
        }

        self.free_pages_len -= 1;
        let ret = unsafe { self.free_pages.add(self.free_pages_len).read() * PAGE_SIZE };
        Ok(ret)
    }
}

/// A [`MemoryTracker`] instance protected behind a [`RawEpochMutex`].
///
/// # Epoch Counter
///
/// Using an epoch-based mutex allows us to avoid having to perform any atomic stores when reading
/// the page list. This is required because userspace processes cannot write to the page list
/// (it is mapped as read-only into their address space).
#[repr(C)]
pub struct LockedMemoryTracker {
    inner: UnsafeCell<MemoryTracker>,
    epoch: RawEpochMutex,
}

unsafe impl Sync for LockedMemoryTracker {}
unsafe impl Send for LockedMemoryTracker {}

impl LockedMemoryTracker {
    /// Creates a new [`LockedMemoryTracker`] instance.
    #[inline(always)]
    pub const fn new(page_list: MemoryTracker) -> Self {
        Self {
            inner: UnsafeCell::new(page_list),
            epoch: RawEpochMutex::UNLOCKED,
        }
    }

    /// Locks the [`LockedMemoryTracker`] instance and returns a guard that allows access to it.
    ///
    /// When the guard is dropped, the [`LockedMemoryTracker`] is unlocked automatically.
    #[inline(always)]
    pub fn lock(&self) -> MemoryTrackerGuard {
        self.epoch.lock();
        MemoryTrackerGuard {
            page_list: unsafe { &mut *self.inner.get() },
            lock: &self.epoch,
        }
    }
}

/// A guard that allows access to a [`MemoryTracker`] instance.
pub struct MemoryTrackerGuard<'a> {
    page_list: &'a mut MemoryTracker,
    lock: &'a RawEpochMutex,
}

impl<'a> Deref for MemoryTrackerGuard<'a> {
    type Target = MemoryTracker;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.page_list
    }
}

impl<'a> DerefMut for MemoryTrackerGuard<'a> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.page_list
    }
}

impl<'a> Drop for MemoryTrackerGuard<'a> {
    #[inline(always)]
    fn drop(&mut self) {
        // SAFETY:
        //  The existence of the guard ensures that we hold the lock.
        unsafe { self.lock.unlock() };
    }
}

/// The global memory tracker instance.
static mut MEMORY_TRACKER: MaybeUninit<LockedMemoryTracker> = MaybeUninit::uninit();

/// A "token" type that allows access to the global page list.
#[derive(Debug, Clone, Copy)]
pub struct MemoryTrackerTok(());

impl MemoryTrackerTok {
    /// Creates a new [`MemoryTrackerTok`] instance.
    ///
    /// # Safety
    ///
    /// The global page list must have been initialized.
    #[inline(always)]
    pub const unsafe fn unchecked() -> Self {
        Self(())
    }

    /// Creates a new [`MemoryTrackerTok`] instance by initializing the global memory tracker.
    ///
    /// # Safety
    ///
    /// This function must only be called once.
    #[inline(always)]
    pub unsafe fn init(page_list: MemoryTracker) -> Self {
        unsafe {
            MEMORY_TRACKER.write(LockedMemoryTracker::new(page_list));
            Self::unchecked()
        }
    }
}

impl Deref for MemoryTrackerTok {
    type Target = LockedMemoryTracker;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        // SAFETY:
        //  The token ensures that the global memory tracker has been initialized.
        unsafe { MEMORY_TRACKER.assume_init_ref() }
    }
}

impl DerefMut for MemoryTrackerTok {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY:
        //  The token ensures that the global memory tracker has been initialized.
        unsafe { MEMORY_TRACKER.assume_init_mut() }
    }
}
