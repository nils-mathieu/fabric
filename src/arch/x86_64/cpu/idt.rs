use core::mem::size_of;
use core::ptr::addr_of;

use super::gdt;
use crate::log;
use crate::x86_64::cpu::apic;
use crate::x86_64::cpu::gdt::DOUBLE_FAULT_STACK_INDEX;
use crate::x86_64::raw;
use crate::x86_64::raw::GateFlags;

// LAPIC interrupt vector offsets.

pub const LAPIC_SPURIOUS_VECTOR: usize = 0x64;
pub const LAPIC_TIMER_VECTOR: usize = 0x65;

// CPU exception offsets in the IDT.

pub const DIVISION_ERROR: usize = 0;
pub const DEBUG: usize = 1;
pub const NON_MASKABLE_INTERRUPT: usize = 2;
pub const BREAKPOINT: usize = 3;
pub const OVERFLOW: usize = 4;
pub const BOUND_RANGE_EXCEEDED: usize = 5;
pub const INVALID_OPCODE: usize = 6;
pub const DEVICE_NOT_AVAILABLE: usize = 7;
pub const DOUBLE_FAULT: usize = 8;
pub const INVALID_TSS: usize = 10;
pub const SEGMENT_NOT_PRESENT: usize = 11;
pub const STACK_SEGMENT_FAULT: usize = 12;
pub const GENERAL_PROTECTION_FAULT: usize = 13;
pub const PAGE_FAULT: usize = 14;
pub const X87_FLOATING_POINT: usize = 16;
pub const ALIGNMENT_CHECK: usize = 17;
pub const MACHINE_CHECK: usize = 18;
pub const SIMD_FLOATING_POINT: usize = 19;
pub const VIRTUALIZATION: usize = 20;
pub const CONTROL_PROTECTION: usize = 21;
pub const HYPERVISOR_INJECTION: usize = 28;
pub const VMM_COMMUNICATION: usize = 29;
pub const SECURITY: usize = 30;

static mut IDT: [[u64; 2]; 256] = [[0, 0]; 256];

static IDT_DESC: raw::TableDesc = raw::TableDesc {
    base: unsafe { addr_of!(IDT) as *const () },
    limit: size_of::<[[u64; 2]; 256]>() as u16 - 1,
};

/// Creates a new gate descriptor.
///
/// # Notes
///
/// - `ist` should be a value between 0 and 7, inclusive.
fn create_gate(dissable_interrupts: bool, offset: u64, ist: usize) -> [u64; 2] {
    debug_assert!(ist <= 7);

    let mut low = 0;
    let mut high = 0;

    high |= offset >> 32;
    low |= (offset & 0xFFFF_0000) << 32;
    low |= offset & 0xFFFF;
    low |= GateFlags::PRESENT.bits();
    low |= (ist as u64) << 32;

    if dissable_interrupts {
        low |= GateFlags::INTERRUPT_GATE.bits();
    } else {
        low |= GateFlags::TRAP_GATE.bits();
    }

    low |= (gdt::KERNEL_CODE_SELECTOR as u64) << 16;

    [low, high]
}

/// Creates an gate descriptor suitable for CPU exceptions.
#[inline(always)]
fn trap_gate(offset: u64) -> [u64; 2] {
    create_gate(false, offset, 0)
}

/// Creates a gate descriptor suitable for interrupts.
#[inline(always)]
fn interrupt_gate(offset: u64) -> [u64; 2] {
    create_gate(true, offset, 0)
}

/// Initializes an Interrupt Descriptor Table for the kernel.
///
/// # Safety
///
/// This function must only be called once.
#[inline] // only called once
pub unsafe fn init() {
    #[allow(clippy::fn_to_numeric_cast)]
    unsafe {
        use super::exceptions::*;

        IDT[DIVISION_ERROR] = trap_gate(division_error as u64);
        IDT[DEBUG] = trap_gate(debug as u64);
        IDT[NON_MASKABLE_INTERRUPT] = trap_gate(non_maskable_interrupt as u64);
        IDT[BREAKPOINT] = trap_gate(breakpoint as u64);
        IDT[OVERFLOW] = trap_gate(overflow as u64);
        IDT[BOUND_RANGE_EXCEEDED] = trap_gate(bound_range_exceeded as u64);
        IDT[INVALID_OPCODE] = trap_gate(invalid_opcode as u64);
        IDT[DEVICE_NOT_AVAILABLE] = trap_gate(device_not_available as u64);
        IDT[DOUBLE_FAULT] = create_gate(false, double_fault as u64, DOUBLE_FAULT_STACK_INDEX + 1);
        IDT[INVALID_TSS] = trap_gate(invalid_tss as u64);
        IDT[SEGMENT_NOT_PRESENT] = trap_gate(segment_not_present as u64);
        IDT[STACK_SEGMENT_FAULT] = trap_gate(stack_segment_fault as u64);
        IDT[GENERAL_PROTECTION_FAULT] = trap_gate(general_protection_fault as u64);
        IDT[PAGE_FAULT] = trap_gate(page_fault as u64);
        IDT[X87_FLOATING_POINT] = trap_gate(x87_floating_point as u64);
        IDT[ALIGNMENT_CHECK] = trap_gate(alignment_check as u64);
        IDT[MACHINE_CHECK] = trap_gate(machine_check as u64);
        IDT[SIMD_FLOATING_POINT] = trap_gate(simd_floating_point as u64);
        IDT[VIRTUALIZATION] = trap_gate(virtualization as u64);
        IDT[CONTROL_PROTECTION] = trap_gate(control_protection as u64);
        IDT[HYPERVISOR_INJECTION] = trap_gate(hypervisor_injection as u64);
        IDT[VMM_COMMUNICATION] = trap_gate(vmm_communication as u64);
        IDT[SECURITY] = trap_gate(security as u64);

        IDT[LAPIC_SPURIOUS_VECTOR] = interrupt_gate(apic::spurious_interrupt as u64);
        IDT[LAPIC_TIMER_VECTOR] = interrupt_gate(apic::timer as u64);
    }

    log::trace!("Switching IDT...");

    unsafe {
        core::arch::asm!(
            r#"
            lidt [{idt_desc}]
            "#,
            idt_desc = in(reg) &IDT_DESC,
            options(preserves_flags, nostack),
        );
    }
}
