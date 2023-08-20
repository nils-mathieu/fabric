//! This module wraps the functions required to handle the `syscall` instruction.

use core::arch::asm;

use crate::log;

use fabric_sys::SysResult;

mod handlers;

use super::instr::{rdmsr, wrmsr};
use super::kernel_stack::KERNEL_STACK_TOP;
use super::mem::HHDM_OFFSET;
use super::raw;

/// The type of a system call handler.
type SystemCallFn = extern "C" fn(usize, usize, usize, usize, usize, usize) -> SysResult;

/// The total number of system calls.
const SYSTEM_CALL_COUNT: usize = 5;

/// A lookup table of system call handlers.
///
/// The functions specified in this table are called in the [`system_call`] function. Attempting
/// to perform a system call with an invalid index should always return the
/// [`SysResult::INVALID_VALUE`] error.
static SYSTEM_CALLS: [SystemCallFn; SYSTEM_CALL_COUNT] = [
    handlers::terminate,
    handlers::map_memory,
    handlers::unmap_memory,
    handlers::acquire_framebuffer,
    handlers::release_framebuffer,
];

/// The function that is called when a userspace program executes the `syscall` instruction.
///
/// # Arguments
///
/// The meaning of arguments to this function depend on the value of the `rax` register.
///
/// Arguments are passed in the following registers:
///
/// - `rdi`
/// - `rsi`
/// - `rdx`
/// - `r10`
/// - `r8`
/// - `r9`
///
/// These registers have been choosen to match the calling convention used by the C language.
/// One exception to this is the `rcx` register, which has been replaced by `r10`. This is because
/// the `syscall` instruction uses the `rcx` register to store the return address, and we need to
/// save it on the stack before calling the system call handler. For this reason, we use `r10` to
/// pass the sixth argument.
///
/// # Returns
///
/// The return value of this function is passed in the `rax` register.
///
/// # Safety
///
/// By nature, this function is always safe to call.
///
/// # Clobbered Registers
///
/// The same rules as for the C calling convention apply.
#[naked]
extern "C" fn system_call() {
    unsafe {
        // The `syscall` instruction invoked by the userland program puts the return address in
        // the `rcx` register. We need to save it on the stack before clobbering all the registers
        // by calling the system call handler.
        //
        // Note that system calls must not touch the stack of the caller, as it might be invalid
        // or broken. Instead, we need to use our own stack. The stack pointer of the caller is
        // saved on the kernel stack, and will be restored before returning with `sysretq`.
        //
        // We're calling a C function, which writes the return value in the `rax` register. Our
        // system calls also return the value in `rax`, so we don't need to do anything more than
        // calling the function.
        asm!(
            r#"
            cmp rax, {syscall_count}
            jae 2f

            mov r12, {stack_offset}
            add r12, [{kernel_stack_top}]
            mov [r12], rsp
            mov rsp, r12

            push rbp
            mov rbp, rsp
            push rcx
            mov rcx, r10

            call [{system_calls} + 8 * rax]

            pop rcx
            pop rbp
            pop rsp
            sysretq

        2:
            mov rax, {invalid_syscall_number}
            sysretq
            "#,
            kernel_stack_top = sym KERNEL_STACK_TOP,
            // The `kernel_stack_top` symbol is the physical address of the top of the kernel stack.
            // We need to offset it by `HHDM_OFFSET` to access it through virtual memory. However,
            // the current stack pointer must be pushed on the stack before we can update it, so we
            // need to subtract 8 bytes from the offset, which is the size of a pointer.
            stack_offset = const HHDM_OFFSET - 8,
            syscall_count = const SYSTEM_CALL_COUNT,
            system_calls = sym SYSTEM_CALLS,
            invalid_syscall_number = const SysResult::INVALID_VALUE.0,
            options(noreturn),
        )
    }
}

/// Initializes the system call handler.
///
/// # Safety
///
/// This function must only be called once.
#[inline] // only called once
#[allow(clippy::assertions_on_constants)] // must remain true
pub unsafe fn init() {
    log::trace!("Initializing the system call handler...");

    #[cfg(debug_assertions)]
    {
        use fabric_sys::x86_64::Syscall::*;
        use handlers::*;
        use SYSTEM_CALLS as TAB;

        assert_eq!(TAB[Terminate as usize], terminate as _);
        assert_eq!(TAB[MapMemory as usize], map_memory as _);
        assert_eq!(TAB[UnmapMemory as usize], unmap_memory as _);
        assert_eq!(TAB[AcquireFramebuffer as usize], acquire_framebuffer as _);
        assert_eq!(TAB[ReleaseFramebuffer as usize], release_framebuffer as _);
    }

    // Intel processors normally use **SYSENTER** and **SYSEXIT** instructions to perform system
    // calls. However, Intel also provide a way to use the **SYSCALL** and **SYSRET** instructions
    // instead. This is what we're going to use, because that allows us to be compatible with AMD
    // processors.
    unsafe {
        let mut efer = raw::Efer::from_bits_retain(rdmsr(raw::IA32_EFER));
        efer.insert(raw::Efer::SYSCALL_ENABLE);
        wrmsr(raw::IA32_EFER, efer.bits());
    }

    // Specify the address of the system call handler.
    // The process will jump to this virtual address when the **SYSCALL** instruction is executed.
    unsafe { wrmsr(raw::LSTAR, system_call as usize as u64) };

    // Specify the code segment and data segment to use when executing the **SYSCALL** and
    // **SYSRET** instructions.
    use super::cpu::gdt::{KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR};
    use super::cpu::gdt::{USER_CODE_SELECTOR, USER_DATA_SELECTOR};

    // This constant specifies the segment selectors that will be loaded when the **SYSRET**
    // instruction is loaded.
    // The CS register will be set to this value plus 16. And the SS register will be set to this
    // value plus 8.
    const SYSRET_BASE: u16 = USER_CODE_SELECTOR - 2 * 8;
    debug_assert!(USER_CODE_SELECTOR & 0b11 == 0b11);
    debug_assert!(USER_DATA_SELECTOR & 0b11 == 0b11);
    debug_assert!(USER_CODE_SELECTOR == SYSRET_BASE + 16);
    debug_assert!(USER_DATA_SELECTOR == SYSRET_BASE + 8);

    // This constant specifies the segment selectors that will be loaded when the **SYSCALL**
    // instruction is loaded.
    // The CS register will be set to this value, and the SS register will be set to this value
    // plus 8.
    const SYSCALL_BASE: u16 = KERNEL_CODE_SELECTOR;
    debug_assert!(KERNEL_CODE_SELECTOR & 0b11 == 0b00);
    debug_assert!(KERNEL_DATA_SELECTOR & 0b11 == 0b00);
    debug_assert!(KERNEL_CODE_SELECTOR == SYSCALL_BASE);
    debug_assert!(KERNEL_DATA_SELECTOR == SYSCALL_BASE + 8);

    unsafe {
        wrmsr(
            raw::STAR,
            (SYSCALL_BASE as u64) << 32 | (SYSRET_BASE as u64) << 48,
        );
    }
}
