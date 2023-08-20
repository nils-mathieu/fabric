use core::mem::size_of;

use fabric_sys::x86_64::public::{Framebuffer, PublicData};

/// Stores the layout of the public data area mapped in every userspace program.
///
/// # Layout
///
/// The memory layout of the public data area is as follows:
///
/// ||
/// |-|
/// | An instance of [`PublicData`]                    |
/// | `framebuffer_count` instances of [`Framebuffer`] |
pub struct PublicDataLayout {
    /// The total size of the public data area.
    pub size: usize,
    /// The root pointer to the public data area.
    pub root: usize,
    /// The virtual address at which the framebuffers should be written.
    pub framebuffers: usize,
}

impl PublicDataLayout {
    /// Creates a new [`PublicDataLayout`] for the provided parameters.
    pub fn compute(framebuffer_count: usize) -> Self {
        let mut offset = 0;

        let root = offset;
        offset += size_of::<PublicData>();

        let framebuffers = offset;
        offset += size_of::<Framebuffer>() * framebuffer_count;

        Self {
            size: offset,
            root,
            framebuffers,
        }
    }
}
