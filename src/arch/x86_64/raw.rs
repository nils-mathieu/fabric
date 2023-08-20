#![allow(dead_code)]

use bitflags::bitflags;

bitflags! {
    /// Describes the flags which can be set on a segment descriptor.
    #[derive(Debug, Clone, Copy)]
    pub struct SegmentFlags: u64 {
        /// Indicates that the segment is currently accessed.
        const ACCESSED = 1 << 40;
        /// Indicates that the segment can be read from.
        const READABLE = 1 << 41;
        /// Indicates that the segment can be written to.
        const WRITABLE = 1 << 41;
        /// Indicates that the segment can be accessed from lower privilege levels.
        const CONFORMING = 1 << 42;
        /// Indicates that the segment grows down (when `DATA` is set, and `EXECUTABLE` is not set).
        const EXPAND_DOWN = 1 << 42;
        /// Indicates that the segment is executable.
        const EXECUTABLE = 1 << 43;
        /// Indicates that the segment is not a system segment.
        const DATA = 1 << 44;
        /// Indicates that the segment can be accessed from ring 3.
        const USER = 3 << 45;
        /// Indicates that the segment is present. When not set, the descriptor is ignored.
        const PRESENT = 1 << 47;
        /// Indicates that the segment is a long-mode code segment.
        const LONG_MODE_CODE = 1 << 53;
        /// Indicates that the segment is a 32-bit segment.
        const SIZE_32BIT = 1 << 54;
        /// Indicates that the segment has a 4KiB granularity. Otherwise, it has byte granularity.
        ///
        /// This changes the meaning of the `limit` field.
        const GRANULARITY = 1 << 55;
        /// Indicates that the segment refers to a Task State Segment.
        ///
        /// Note that this is only valid for system segments (without the [`DescriptorFlags::DATA`] flag).
        const AVAILABLE_TSS = 0x9 << 40;
        /// Not really a flag, but this sets the limit to the maximum value (0xFFFFF).
        const LIMIT_MAX = 0x000F00000000FFFF;
    }
}

bitflags! {
    /// Describes which flags can be set on a gate descriptor.
    #[derive(Debug, Clone, Copy)]
    pub struct GateFlags: u64 {
        /// Indicates that the segment is currently accessed.
        const PRESENT = 1 << 47;
        /// The segment is an interrupt gate, it will disable interrupts when entered.
        const INTERRUPT_GATE = 0b1110 << 40;
        /// The segment is a trap gate, it will not disable interrupts when entered.
        const TRAP_GATE = 0b1111 << 40;
    }
}

/// The content of a table-like register, used by instructions like **LGDT** or **LIDT**.
#[repr(C, packed)]
pub struct TableDesc {
    pub limit: u16,
    pub base: *const (),
}

unsafe impl Send for TableDesc {}
unsafe impl Sync for TableDesc {}

/// The content of a Task State Segment.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed(4))]
pub struct TaskStateSegment {
    pub reserved0: u32,
    pub privilege_stack_table: [u64; 3],
    pub reserved1: u64,
    pub interrupt_stack_table: [u64; 7],
    pub reserved2: u64,
    pub reserved3: u16,
    pub iomap_base: u16,
}

bitflags! {
    /// The flags allowed in page table entries.
    #[derive(Debug, Clone, Copy)]
    pub struct PageFlags: u64 {
        /// Indicates that the page is present. When this bit is not set, the entry is ignored.
        const PRESENT = 1 << 0;
        /// Indicates that the page is writable. Otherwise, it is read-only.
        const WRITABLE = 1 << 1;
        /// Indicates that the page can be accessed by user-mode code. Otherwise, it may only be
        /// accessed from ring 0.
        const USER = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const DISABLE_CACHE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        /// When set in a page directory entry, indicates that the entry actually refers to a
        /// huge page.
        ///
        /// The size of the huge page differs depending on the page table level.
        const HUGE = 1 << 7;
        /// Indicates that the page will remain in all address spaces.
        const GLOBAL = 1 << 8;
        /// Indicates that code cannot be executed from the page.
        const NO_EXECUTE = 1 << 63;
    }
}

/// The **IA32_EFER** model-specific register.
///
/// The *Extended Feature Enable Register* is used on Intel processors to enable certain features
/// of the CPU. It is used to use the **SYSCALL** and **SYSRET** instructions for compatibility
/// with AMD processors.
pub const IA32_EFER: u32 = 0xC000_0080;
/// The **STAR** model-specific register.
///
/// It stores the segment selectors that will be loaded when the **SYSCALL** and **SYSRET**
/// instructions are invoked.
pub const STAR: u32 = 0xC000_0081;
/// The **LSTAR** model-specific register.
///
/// It stores the address of the system call handler.
pub const LSTAR: u32 = 0xC000_0082;

bitflags! {
    /// The flags allowed in the **IA32_EFER** model-specific register.
    pub struct Efer: u64 {
        /// Enables the **SYSCALL** and **SYSRET** instructions, for compatibility with AMD
        /// processors.
        const SYSCALL_ENABLE = 1 << 0;
    }
}

/// The **IA32_APIC_BASE** model-specific register.
///
/// This register contains the base physical address of the local APIC.
pub const IA32_APIC_BASE: u32 = 0x1B;

pub const LAPIC_EOI: usize = 0x0B0;
pub const LAPIC_TIMER_INTERRUPT_VECTOR: usize = 0x320;
pub const LAPIC_SPURIOUS_INTERRUPT_VECTOR: usize = 0x0F0;
pub const LAPIC_INITIAL_COUNT: usize = 0x380;
pub const LAPIC_CURRENT_COUNT: usize = 0x390;
pub const LAPIC_DIVIDE_CONFIG: usize = 0x3E0;

#[repr(C)]
pub struct StackFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

// LAPIC timer configurations.

pub const LAPIC_TIMER_ONE_SHOT: u32 = 0 << 17;
pub const LAPIC_TIMER_PERIODIC: u32 = 1 << 17;
pub const LAPIC_TIMER_TSC_DEADLINE: u32 = 2 << 17;

// LAPIC divide configurations.

pub const LAPIC_DIVIDE_BY_2: u32 = 0;
const LAPIC_DIVIDE_BY_4: u32 = 1;
pub const LAPIC_DIVIDE_BY_8: u32 = 2;
pub const LAPIC_DIVIDE_BY_16: u32 = 3;
pub const LAPIC_DIVIDE_BY_32: u32 = 8;
pub const LAPIC_DIVIDE_BY_64: u32 = 9;
pub const LAPIC_DIVIDE_BY_128: u32 = 10;
pub const LAPIC_DIVIDE_BY_256: u32 = 11;
