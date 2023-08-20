//! # Fabric
//!
//! Fabric is an experimental [exokernel][1].
//!
//! It tries to give untrusted applications as much control over the hardware as possible without
//! sacrificing security.
//!
//! The present documentation describes the kernel's architecture and the design decisions that
//! were made.
//!
//! ## Goals
//!
//! - **Security**: Untrusted applications shouldn't be able to access resources they don't own.
//! - **Performance**: As much as possible, the kernel should attempt not to get in the way of
//!   applications. Accessing hardware should be as low overhead as possible.
//!
//! ## Non-goals
//!
//! - **Portability**: By design, Fabric cannot be portable. Its interface is very specific to the
//!   hardware it runs on.
//!
//! ## Supported Architectures
//!
//! Fabric currently only have support for the **x86_64** architecture. Documentation specific for
//! this architecture can be found in the [`x86_64`] module.
//!
//! [1]: https://en.wikipedia.org/wiki/Exokernel

#![no_std]
#![no_main]
//
#![deny(unsafe_op_in_unsafe_fn)]
//
#![feature(naked_functions)]
#![feature(asm_const)]
#![feature(abi_x86_interrupt)]
#![feature(decl_macro)]
#![feature(panic_info_message)]
#![feature(pointer_byte_offsets)]

mod builtins;
mod log;
mod utility;

#[path = "arch/x86_64/mod.rs"]
#[cfg(target_arch = "x86_64")]
mod x86_64;

/// Efficiently halts the CPU forever without spinning.
#[inline(always)]
fn die() -> ! {
    #[cfg(target_arch = "x86_64")]
    self::x86_64::die();
}

/// This function is called when something goes wrong in the kernel.
///
/// This should *never* happen, and is if the control flow ever goes through this function, it
/// means that there is a bug in the kernel.
///
/// When an expected error occurs, but the kernel cannot recover from it, the kernel should *not*
/// panic, and instead hang or reboot the machine.
#[panic_handler]
fn bug(info: &core::panic::PanicInfo) -> ! {
    log::error!("KERNEL PANIC!");
    log::error!("");
    log::error!("  This is a serious bug in the kernel.");
    log::error!("  Please report this issue at:");
    log::error!("");
    log::error!("      https://github.com/nils-mathieu/fabric/issues/new");
    log::error!("");
    log::error!("  Additional Information:");
    match info.message() {
        Some(msg) => log::error!("   > Message  = \"{msg}\""),
        None => log::error!("   > Message  = <no message>"),
    }
    match info.location() {
        Some(loc) => log::error!(
            "   > Location = {}:{}:{}",
            loc.file(),
            loc.line(),
            loc.column()
        ),
        None => log::error!("   > Location = <no location>"),
    }

    die();
}
