use core::arch::asm;

use crate::log;
use crate::x86_64::raw::StackFrame;

pub extern "x86-interrupt" fn division_error(_stack_frame: StackFrame) {
    panic!("Division Error");
}

pub extern "x86-interrupt" fn debug(_stack_frame: StackFrame) {
    panic!("Debug Exception");
}

pub extern "x86-interrupt" fn non_maskable_interrupt(_stack_frame: StackFrame) {
    panic!("Non Maskable Interrupt");
}

pub extern "x86-interrupt" fn breakpoint(_stack_frame: StackFrame) {
    log::info!("Breakpoint Exception");
}

pub extern "x86-interrupt" fn overflow(_stack_frame: StackFrame) {
    panic!("Overflow");
}

pub extern "x86-interrupt" fn bound_range_exceeded(_stack_frame: StackFrame) {
    panic!("Bound Range Exceeded");
}

pub extern "x86-interrupt" fn invalid_opcode(stack_frame: StackFrame) {
    panic!("Invalid Opcode (RIP = {:#x})", stack_frame.rip);
}

pub extern "x86-interrupt" fn device_not_available(_stack_frame: StackFrame) {
    panic!("Device Not Available");
}

pub extern "x86-interrupt" fn double_fault(_stack_frame: StackFrame, _error_code: u64) -> ! {
    panic!("Double Fault");
}

pub extern "x86-interrupt" fn invalid_tss(_stack_frame: StackFrame, _error_code: u64) {
    panic!("Invalid TSS");
}

pub extern "x86-interrupt" fn segment_not_present(_stack_frame: StackFrame, _error_code: u64) {
    panic!("Segment Not Present");
}

pub extern "x86-interrupt" fn stack_segment_fault(_stack_frame: StackFrame, _error_code: u64) {
    panic!("Stack Segment Fault");
}

pub extern "x86-interrupt" fn general_protection_fault(_stack_frame: StackFrame, _error_code: u64) {
    panic!("General Protection Fault");
}

pub extern "x86-interrupt" fn page_fault(stack_frame: StackFrame, error_code: u64) {
    let addr: usize;
    unsafe {
        asm!("mov {}, cr2", out(reg) addr, options(nostack, nomem, preserves_flags));
    }

    panic!(
        "Page Fault (RIP = {:#x}, RSP = {:#x}, addr = {:#x}, error = {:#b})",
        stack_frame.rip, stack_frame.rsp, addr, error_code,
    );
}

pub extern "x86-interrupt" fn x87_floating_point(_stack_frame: StackFrame) {
    panic!("x87 Floating Point");
}

pub extern "x86-interrupt" fn alignment_check(_stack_frame: StackFrame, _error_code: u64) {
    panic!("Alignment Check");
}

pub extern "x86-interrupt" fn machine_check(_stack_frame: StackFrame) -> ! {
    panic!("Machine Check");
}

pub extern "x86-interrupt" fn simd_floating_point(_stack_frame: StackFrame) {
    panic!("SIMD Floating Point");
}

pub extern "x86-interrupt" fn virtualization(_stack_frame: StackFrame) {
    panic!("Virtualization");
}

pub extern "x86-interrupt" fn control_protection(_stack_frame: StackFrame, _error_code: u64) {
    panic!("Control Protection");
}

pub extern "x86-interrupt" fn hypervisor_injection(_stack_frame: StackFrame) {
    panic!("Hypervisor Injection");
}

pub extern "x86-interrupt" fn vmm_communication(_stack_frame: StackFrame, _error_code: u64) {
    panic!("Security Exception");
}

pub extern "x86-interrupt" fn security(_stack_frame: StackFrame, _error_code: u64) {
    panic!("Security Exception");
}
