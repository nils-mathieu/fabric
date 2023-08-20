use core::sync::atomic::AtomicUsize;

/// A color mode available for framebuffers.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /// The framebuffers uses three bytes to represent each pixel.
    ///
    /// The first byte is the amount of red light for the pixel, the second byte is the amount of
    /// green light, and the last one is the amount of blue light.
    Rgb24,
    /// The framebuffer uses four bytes to represent each pixel.
    ///
    /// The first byte is the amount of red light for the pixel, the second byte is the amount of
    /// green light, the third byte is the amount of blue light, and the last byte is either unused
    /// or the opacity value of the pixel.
    Rgb32,
}

/// Information about a framebuffer.
#[repr(C)]
#[derive(Debug)]
pub struct Framebuffer {
    /// The physical address of the framebuffer's in-memory buffer.
    pub physical_address: usize,
    /// The width of the framebuffer, in pixels.
    pub width: usize,
    /// The height of the framebuffer, in pixels.
    pub height: usize,
    /// The number of bytes taken by each row of the frame buffer.
    pub pitch: usize,
    /// The color mode of the framebuffer.
    pub color_mode: ColorMode,

    pub _reserved: [u8; 7],

    /// The ID of the process owned by the framebuffer, if any.
    ///
    /// When non zero, the framebuffer is in use by the process with the given ID. When `0`,
    /// the framebuffer is not being used.
    pub owned_by: AtomicUsize,
}

impl Framebuffer {
    /// Returns the size of the framebuffer's in-memory buffer, in bytes.
    #[inline(always)]
    pub fn size_in_bytes(&self) -> usize {
        self.pitch * self.height
    }
}
