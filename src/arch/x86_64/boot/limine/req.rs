use core::ffi::c_char;
use core::ptr::addr_of;

use super::raw;
use crate::x86_64::mem::PAGE_SIZE;
use crate::{builtins, log};

/// A "token" type that proves that the bootloader reclaimable memory map is still around
/// and properly mapped.
#[derive(Clone, Copy)]
pub struct LimineTok<'a> {
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> LimineTok<'a> {
    /// Creates a new [`LimineTok`] token from nothing.
    ///
    /// # Safety
    ///
    /// The created [`LimineTok<'a>`] token must be tied to the lifetime of the bootloader
    /// reclaimable memory (or weaker).
    ///
    /// You should also understand that the functions that require a [`LimineTok`] token can
    /// assume that the bootloader implementation is correct. If the bootloader responds with
    /// an invalid pointer, or writes a pointer to arbitrary memory, the behavior is undefined.
    #[inline(always)]
    pub const fn unchecked() -> Self {
        Self {
            _marker: core::marker::PhantomData,
        }
    }
}

/// This symbol is placed in the `.limine_reqs` section of the kernel ELF image. Limine bootloaders
/// will look for this section and use the information it contains to detect the requests made by
/// the kernel.
///
/// The linker script must **KEEP** this section in the final kernel image as it won't visibly be
/// used anywhere else in the image.
#[link_section = ".limine_reqs"]
#[used]
static mut LIMINE_REQS: [*const (); 8] = unsafe {
    [
        addr_of!(BOOTLOADER_INFO) as *const (),
        addr_of!(HHDM) as *const (),
        addr_of!(FRAMEBUFFER) as *const (),
        addr_of!(MEMMAP) as *const (),
        addr_of!(ENTRY_POINT) as *const (),
        addr_of!(MODULE) as *const (),
        addr_of!(KERNEL_ADDRESS) as *const (),
        core::ptr::null(),
    ]
};

static mut BOOTLOADER_INFO: raw::BootloaderInfoRequest = raw::BootloaderInfoRequest {
    id: raw::BOOTLOADER_INFO_REQUEST,
    revision: raw::BOOTLOADER_INFO_REQUEST_REVISION,
    response: raw::ResponsePtr::NULL,
};

/// Logs information about the bootloader that responded to the requests.
pub fn log_bootloader_info(_: LimineTok) {
    // Not responding to this request is pretty weird, but the specification does not mandate that
    // it must though.

    // SAFETY:
    //  Requests are never accessed multably.
    let bootloader_info_response = unsafe { BOOTLOADER_INFO.response.read() };

    if bootloader_info_response.is_null() {
        log::info!("Loaded by a Limine-complient bootloader.");
    } else {
        // SAFETY:
        //  The `LimineTok` token that this function requires proves that the bootloader reclaimable
        //  memory is still mapped and initialized.
        let response = unsafe { &*bootloader_info_response };

        // The specification requires those fields to contain valid ASCII strings. If one of those
        // strings are not valid UTF-8, the `escape_ascii` function will replace the invalid
        // characters with antislash-style escape sequences.
        //
        // SAFETY:
        //  This relies on the correctness of the bootloader. If one of those strings
        //  are not null-terminated, this triggers undefined behavior. This is *acceptable*
        //  potential UB :)
        let name = unsafe { make_u8_slice(response.name).escape_ascii() };
        let version = unsafe { make_u8_slice(response.version).escape_ascii() };

        log::info!("Loaded by '{name}' (version '{version}')")
    }
}

static mut HHDM: raw::HhdmRequest = raw::HhdmRequest {
    id: raw::HHDM_REQUEST,
    revision: raw::HHDM_REQUEST_REVISION,
    response: raw::ResponsePtr::NULL,
};

/// Returns the higher half direct map address computed by the bootloader.
pub fn hhdm_offset(_: LimineTok) -> usize {
    // SAFETY:
    //  This request is never accessed mutably.
    let response = unsafe { HHDM.response.read() };
    if response.is_null() {
        log::error!("The bootloader did not provide the higher half direct map offset.");
        crate::die();
    }

    // SAFETY:
    //  The `LimineTok` token that this function requires proves that the bootloader reclaimable
    //  memory is still mapped and initialized.
    let response = unsafe { &*response };

    response.offset as usize
}

static mut ENTRY_POINT: raw::EntryPointRequest = raw::EntryPointRequest {
    id: raw::ENTRY_POINT_REQUEST,
    revision: raw::ENTRY_POINT_REQUEST_REVISION,
    response: raw::ResponsePtr::NULL,
    entry_point: super::entry_point,
};

/// Validates the entry point request made by the kernel to the bootloader.
///
/// If an inconsistency is detected, the function will print a warning or die depending on the
/// sevierity of the issue.
pub fn validate_entry_point(_: LimineTok) {
    // SAFETY:
    //  This request is never accessed mutably.
    let entry_point_response = unsafe { ENTRY_POINT.response.read() };
    if entry_point_response.is_null() {
        log::warn!("The bootloader did not respond to the entry point request.");
        log::warn!("It is likely that the bootloader is not fully Limine-complient.");
    }
}

static mut FRAMEBUFFER: raw::FramebufferRequest = raw::FramebufferRequest {
    id: raw::FRAMEBUFFER_REQUEST,
    revision: raw::FRAMEBUFFER_REQUEST_REVISION,
    response: raw::ResponsePtr::NULL,
};

/// Returns a list of framebuffers provided by the bootloader.
pub fn framebuffers(_: LimineTok) -> &[&raw::Framebuffer] {
    // SAFETY:
    //  This request is never accessed mutably.
    let response = unsafe { FRAMEBUFFER.response.read() };
    if response.is_null() {
        log::warn!("The bootloader did not respond with a framebuffer list.");
        return &[];
    }

    // SAFETY:
    //  The `LimineTok` token that this function requires proves that the bootloader reclaimable
    //  memory is still mapped and initialized.
    let response = unsafe { &*response };

    let framebuffers = unsafe {
        core::slice::from_raw_parts(
            response.framebuffers as *const &raw::Framebuffer,
            response.framebuffer_count as usize,
        )
    };

    framebuffers
}

static mut MEMMAP: raw::MemMapRequest = raw::MemMapRequest {
    id: raw::MEMMAP_REQUEST,
    revision: raw::MEMMAP_REQUEST_REVISION,
    response: raw::ResponsePtr::NULL,
};

/// Returns a slice over the memory map provided by the bootloader.
///
/// # Safety
///
/// The bootloader reclaimable memory must still be mapped in the higher half direct map setup by
/// the bootloader, and must remain mapped for the lifetime of the returned slice (`'a`).
///
/// # Dies
///
/// This function dies if the bootloader did not respond to the memory map request.
pub fn memory_map(_: LimineTok) -> &[&raw::MemMapEntry] {
    // SAFETY:
    //  This request is never accessed mutably.
    let response = unsafe { MEMMAP.response.read() };
    if response.is_null() {
        log::error!("The bootloader did not provide a map of the physical memory.");
        crate::die();
    }

    // SAFETY:
    //  The `LimineTok` token that this function requires proves that the bootloader reclaimable
    //  memory is still mapped and initialized.
    let response = unsafe { &*response };

    // SAFETY:
    //  This relies on the correctness of the bootloader. We don't really have any way to check
    //  whether this is valid or not.
    let memmap = unsafe {
        core::slice::from_raw_parts(
            response.entries as *const &raw::MemMapEntry,
            response.entry_count as usize,
        )
    };

    // The specification requires the bootloader to provide page-aligned usable memory segments.
    // We'll check that to notify the user of an error, but our memory allocator won't rely on
    // that.
    if memmap
        .iter()
        .filter(|e| {
            e.type_ == raw::MEMMAP_USABLE
                || e.type_ == raw::MEMMAP_BOOTLOADER_RECLAIMABLE
                || e.type_ == raw::MEMMAP_ACPI_RECLAIMABLE
        })
        .any(|e| e.base % PAGE_SIZE as u64 != 0 || e.length % PAGE_SIZE as u64 != 0)
    {
        log::warn!("The bootloader did not provide page-aligned usable memory");
        log::warn!("segments. This is not a fatal error, but the bootloader might");
        log::warn!("not be fully Limine-complient.");
    }

    memmap
}

static mut MODULE: raw::ModuleRequest = raw::ModuleRequest {
    id: raw::MODULE_REQUEST,
    revision: raw::MODULE_REQUEST_REVISION,
    response: raw::ResponsePtr::NULL,
    internal_module_count: 1,
    internal_modules: [&raw::InternalModule {
        path: b"fabric_init\0".as_ptr() as *const c_char,
        cmdline: b"".as_ptr() as *const c_char,
        flags: raw::INTERNAL_MODULE_REQUIRED,
    }]
    .as_ptr() as *mut *mut raw::InternalModule,
};

/// Returns a slice over the first module named `fabric_init`.
///
/// # Dies
///
/// This function dies if no such module is found or if the request is not answered by the
/// bootloader.
pub fn fabric_init(_: LimineTok) -> &[u8] {
    // SAFETY:
    //  This request is never accessed mutably.
    let response = unsafe { MODULE.response.read() };

    if response.is_null() {
        log::error!("The bootloader did not respond to the module request.");
        crate::die();
    }

    // SAFETY:
    //  The `LimineTok` token that this function requires proves that the bootloader reclaimable
    // memory is still mapped and initialized.
    let response = unsafe { &*response };

    // SAFETY:
    //  This relies on the correctness of the bootloader. We can't really check that.
    let modules = unsafe {
        core::slice::from_raw_parts(
            response.modules as *const &raw::File,
            response.module_count as usize,
        )
    };

    log::trace!("Enumerating kernel modules...");

    let file = modules
        .iter()
        .find(|file| {
            // SAFETY:
            //  The bootloader must provide valid C strings. We can't really check that.
            let path = unsafe { make_u8_slice(file.path) };

            // Get the position of the last slash.
            let start = match path.iter().rposition(|&c| c == b'/') {
                Some(pos) => pos + 1,
                None => 0,
            };

            // SAFETY:
            //  This is always valid UTF-8.
            let filename = unsafe { path.get_unchecked(start..) };

            filename == b"fabric_init"
        })
        .unwrap_or_else(|| {
            log::error!("No module named 'fabric_init' was found.");
            log::error!("This module is required for the kernel to boot.");
            log::error!("");
            log::error!("Check your 'limine.cfg' configration!");
            crate::die();
        });

    // SAFETY:
    //  This relies on the correctness of the bootloader. We can't really check that.
    unsafe { core::slice::from_raw_parts(file.address as *const u8, file.size as usize) }
}

static mut KERNEL_ADDRESS: raw::KernelAddressRequest = raw::KernelAddressRequest {
    id: raw::KERNEL_ADDRESS_REQUEST,
    revision: raw::KERNEL_ADDRESS_REQUEST_REVISION,
    response: raw::ResponsePtr::NULL,
};

/// Returns the physical of the kernel.
///
/// This function also prints a warning if the virtual address specified is not the one specified
/// in the linker script.
///
/// # Dies
///
/// This function dies if the bootloader did not respond to the kernel address request.
pub fn kernel_physical_address(_: LimineTok) -> usize {
    // SAFETY:
    //  This request is never accessed mutably.
    let response = unsafe { KERNEL_ADDRESS.response.read() };
    if response.is_null() {
        log::error!("The bootloader did not respond to the kernel address request.");
        crate::die();
    }

    // SAFETY:
    //  The `LimineTok` token that this function requires proves that the bootloader reclaimable
    // memory is still mapped and initialized.
    let response = unsafe { &*response };

    if response.virtual_base != crate::x86_64::image_begin() as u64 {
        log::warn!("The bootloader did not load the kernel at the correct virtual address.");
        log::warn!("How are we even running?");
    }

    response.physical_base as usize
}

/// Creates a new Rust string from a C string.
///
/// # Safety
///
/// `s` must be a valid C string. It must be null terminated, and remain borrowed for the lifetime
/// of the returned string reference.
#[inline(always)]
unsafe fn make_u8_slice<'a>(s: *const c_char) -> &'a [u8] {
    unsafe { core::slice::from_raw_parts(s as *const u8, builtins::strlen(s)) }
}
