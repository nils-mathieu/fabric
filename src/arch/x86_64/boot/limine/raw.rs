//! Provides raw types and constants defined by the Limine protocol.

#![allow(dead_code)]

use core::ffi::{c_char, c_void};

/// The limine bootloader writes the address of a response structure directly into the kernel image
/// after it has been loaded into memory. This prevents Rust from being able to know about the
/// "real" value of the pointer. As far is it is conserned, it will always be null. To prevent
/// potential optimizations from replacing reads to the pointer with a constant null, we use
/// volatile reads to access the pointer.
#[repr(transparent)]
pub struct ResponsePtr<T> {
    inner: *mut T,
}

impl<T> ResponsePtr<T> {
    /// A null [`ResponsePtr`].
    pub const NULL: Self = Self {
        inner: core::ptr::null_mut(),
    };

    /// Reads the pointer using volatile semantics.
    #[inline(always)]
    pub fn read(&self) -> *mut T {
        // SAFETY:
        //  We're reading a regular Rust reference.
        unsafe { core::ptr::read_volatile(&self.inner) }
    }
}

#[repr(C)]
pub struct Uuid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[repr(C)]
pub struct File {
    pub revision: u64,
    pub address: *mut c_void,
    pub size: u64,
    pub path: *mut c_char,
    pub cmdline: *mut c_char,
    pub media_type: u32,
    pub unused: u32,
    pub tftp_ip: u32,
    pub tftp_port: u32,
    pub partition_index: u32,
    pub mbr_disk_id: u32,
    pub gpt_disk_uuid: Uuid,
    pub gpt_part_uuid: Uuid,
    pub part_uuid: Uuid,
}

pub const COMMON_MAGIC: [u64; 2] = [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b];

pub const BOOTLOADER_INFO_REQUEST: [u64; 4] = [
    COMMON_MAGIC[0],
    COMMON_MAGIC[1],
    0xf55038d8e2a1202f,
    0x279426fcf5f59740,
];

pub const BOOTLOADER_INFO_REQUEST_REVISION: u64 = 0;

#[repr(C)]
pub struct BootloaderInfoRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: ResponsePtr<BootloaderInfoResponse>,
}

#[repr(C)]
pub struct BootloaderInfoResponse {
    pub revision: u64,
    pub name: *mut c_char,
    pub version: *mut c_char,
}

pub const HHDM_REQUEST: [u64; 4] = [
    COMMON_MAGIC[0],
    COMMON_MAGIC[1],
    0x48dcf1cb8ad2b852,
    0x63984e959a98244b,
];

pub const HHDM_REQUEST_REVISION: u64 = 0;

#[repr(C)]
pub struct HhdmRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: ResponsePtr<HhdmResponse>,
}

#[repr(C)]
pub struct HhdmResponse {
    pub revision: u64,
    pub offset: u64,
}

pub const FRAMEBUFFER_REQUEST: [u64; 4] = [
    COMMON_MAGIC[0],
    COMMON_MAGIC[1],
    0x9d5827dcd881dd75,
    0xa3148604f6fab11b,
];

pub const FRAMEBUFFER_REQUEST_REVISION: u64 = 0;

#[repr(C)]
pub struct FramebufferRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: ResponsePtr<FramebufferResponse>,
}

#[repr(C)]
pub struct FramebufferResponse {
    pub revision: u64,
    pub framebuffer_count: u64,
    pub framebuffers: *mut *mut Framebuffer,
}

pub const FRAMEBUFFER_RGB: u8 = 1;

#[repr(C)]
pub struct Framebuffer {
    pub address: *mut u8,
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u16,
    pub memory_model: u8,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
    pub unused: [u8; 7],
    pub edid_size: u64,
    pub edid: *mut u8,

    /* response revision >= 1 */
    pub mode_count: u64,
    pub modes: *mut *mut VideoMode,
}

#[repr(C)]
pub struct VideoMode {
    pub pitch: u64,
    pub width: u64,
    pub height: u64,
    pub bpp: u16,
    pub memory_model: u8,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
}

pub const MEMMAP_REQUEST: [u64; 4] = [
    COMMON_MAGIC[0],
    COMMON_MAGIC[1],
    0x67cf3d9d378a806f,
    0xe304acdfc50c3c62,
];

pub const MEMMAP_REQUEST_REVISION: u64 = 0;

#[repr(C)]
pub struct MemMapRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: ResponsePtr<MemMapResponse>,
}

#[repr(C)]
pub struct MemMapResponse {
    pub revision: u64,
    pub entry_count: u64,
    pub entries: *mut *mut MemMapEntry,
}

pub const MEMMAP_USABLE: u32 = 0;
pub const MEMMAP_RESERVED: u32 = 1;
pub const MEMMAP_ACPI_RECLAIMABLE: u32 = 2;
pub const MEMMAP_ACPI_NVS: u32 = 3;
pub const MEMMAP_BAD_MEMORY: u32 = 4;
pub const MEMMAP_BOOTLOADER_RECLAIMABLE: u32 = 5;
pub const MEMMAP_KERNEL_AND_MODULES: u32 = 6;
pub const MEMMAP_FRAMEBUFFER: u32 = 7;

#[repr(C)]
pub struct MemMapEntry {
    pub base: u64,
    pub length: u64,
    pub type_: u32,
}

pub const ENTRY_POINT_REQUEST: [u64; 4] = [
    COMMON_MAGIC[0],
    COMMON_MAGIC[1],
    0x13d86c035a1cd3e1,
    0x2b0caa89d8f3026a,
];

pub const ENTRY_POINT_REQUEST_REVISION: u64 = 0;

pub type EntryPoint = unsafe extern "C" fn() -> !;

#[repr(C)]
pub struct EntryPointRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: ResponsePtr<EntryPointResponse>,
    pub entry_point: EntryPoint,
}

#[repr(C)]
pub struct EntryPointResponse {
    pub revision: u64,
}

pub const MODULE_REQUEST: [u64; 4] = [
    COMMON_MAGIC[0],
    COMMON_MAGIC[1],
    0x3e7e279702be32af,
    0xca1c4f3bd1280cee,
];

pub const MODULE_REQUEST_REVISION: u64 = 1;

pub const INTERNAL_MODULE_REQUIRED: u64 = 1 << 0;

#[repr(C)]
pub struct InternalModule {
    pub path: *const c_char,
    pub cmdline: *const c_char,
    pub flags: u64,
}

#[repr(C)]
pub struct ModuleRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: ResponsePtr<ModuleResponse>,
    pub internal_module_count: u64,
    pub internal_modules: *mut *mut InternalModule,
}

#[repr(C)]
pub struct ModuleResponse {
    pub revision: u64,
    pub module_count: u64,
    pub modules: *mut *mut File,
}

pub const KERNEL_ADDRESS_REQUEST: [u64; 4] = [
    COMMON_MAGIC[0],
    COMMON_MAGIC[1],
    0x71ba76863cc55f63,
    0xb2644a48c516a487,
];

pub const KERNEL_ADDRESS_REQUEST_REVISION: u64 = 0;

#[repr(C)]
pub struct KernelAddressRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: ResponsePtr<KernelAddressResponse>,
}

#[repr(C)]
pub struct KernelAddressResponse {
    pub revision: u64,
    pub physical_base: u64,
    pub virtual_base: u64,
}
