use core::ptr;

use crate::x86_64::cpu::idt;
use crate::x86_64::instr::{rdmsr, wrmsr};
use crate::x86_64::mem::HHDM_OFFSET;
use crate::x86_64::raw;
use crate::x86_64::raw::StackFrame;

/// Reads the local APIC base address from the IA32_APIC_BASE MSR.
///
/// This automatically converts the physical address to a virtual address.
#[inline]
fn get_local_apic_base() -> *mut u32 {
    let base = unsafe { rdmsr(raw::IA32_APIC_BASE) & 0xFFFFF000 };
    (base as usize + HHDM_OFFSET) as *mut u32
}

/// Sends an end-of-interrupt (EOI) signal to the local APIC.
#[inline]
fn send_eoi() {
    let base = get_local_apic_base();

    unsafe {
        ptr::write_volatile(base.byte_add(raw::LAPIC_EOI), 0);
    }
}

/// Initializes the local APIC of the current CPU.
///
/// # Safety
///
/// This function may only be called once per CPU core.
pub unsafe fn init_local_apic() {
    let base = unsafe { rdmsr(raw::IA32_APIC_BASE) & 0xFFFFF000 };
    debug_assert!(
        base & 0xFFFFFFFF != 0,
        "The local APIC is too high in memory."
    );

    // Writing to the IA32_APIC_BASE MSR will hardware-enable the local APIC.
    unsafe { wrmsr(raw::IA32_APIC_BASE, base) };

    let base = (base as usize + HHDM_OFFSET) as *mut u32;

    unsafe {
        // Set an spuriour interrupt handler to software enable the local APIC.
        ptr::write_volatile(
            base.byte_add(raw::LAPIC_SPURIOUS_INTERRUPT_VECTOR),
            idt::LAPIC_SPURIOUS_VECTOR as u32 | (1 << 8),
        );

        // TODO:
        //  Compute the speed of the current CPU using the TSC or emulated PIC.

        // Configure and enable the timer.
        ptr::write_volatile(
            base.byte_add(raw::LAPIC_DIVIDE_CONFIG),
            raw::LAPIC_DIVIDE_BY_16,
        );
        ptr::write_volatile(
            base.byte_add(raw::LAPIC_TIMER_INTERRUPT_VECTOR),
            idt::LAPIC_TIMER_VECTOR as u32 | raw::LAPIC_TIMER_PERIODIC,
        );
        ptr::write_volatile(base.byte_add(raw::LAPIC_INITIAL_COUNT), 0x100000);
    }
}

pub extern "x86-interrupt" fn timer(_: StackFrame) {
    send_eoi();
}

pub extern "x86-interrupt" fn spurious_interrupt(_: StackFrame) {
    // empty.
    // We don't need to send an EOI here, because the interrupt is spurious.
}
