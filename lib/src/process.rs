use core::num::NonZeroUsize;

/// The header of the initial process.
#[repr(C)]
pub struct InitHeader {
    /// The magic number of the initial process. This is supposed to be `<limine>` in ASCII, encoded
    /// in the current endianness.
    pub magic: u64,
    /// A pointer to the first byte of the initial process' image. This is the virtual address at
    /// which the process will be loaded.
    pub image_start: *const (),
    /// The entry point of the initial process.
    pub entry_point: *const (),
}

unsafe impl Send for InitHeader {}
unsafe impl Sync for InitHeader {}

impl InitHeader {
    /// The magic number of the initial process.
    pub const MAGIC: u64 = u64::from_ne_bytes(*b"<limine>");

    /// Creates a new [`InitHeader`] from the provided entry point.
    #[inline(always)]
    pub const fn new(image_start: *const (), entry_point: unsafe extern "C" fn() -> !) -> Self {
        Self {
            magic: Self::MAGIC,
            image_start,
            entry_point: entry_point as *const (),
        }
    }
}

/// The ID of a process.
///
/// No process can have the ID zero, which is why this type simply is a [`NonZeroUsize`].
pub type ProcessId = NonZeroUsize;
