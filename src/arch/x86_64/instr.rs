use core::arch::asm;

/// Writes a single byte to the given I/O port.
///
/// # Safety
///
/// Setting arbitrary ports can violate memory safety.
#[inline(always)]
pub unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Reads a single byte from the give I/O port.
///
/// # Safety
///
/// Reading from arbitrary ports can violate memory safety.
#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
    let ret;
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") ret,
            options(nomem, nostack, preserves_flags)
        );
    }
    ret
}

/// Writes to the specified model-specific register.
///
/// # Safety
///
/// Writing to arbitrary MSRs can violate memory safety.
#[inline(always)]
pub unsafe fn wrmsr(port: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") port,
            in("eax") low,
            in("edx") high,
            options(nostack, preserves_flags)
        );
    }
}

/// Reads the value of the specified model-specific register.
///
/// # Safety
///
/// Can reading arbitrary MSRs violate memory safety? Not sure about that.
#[inline(always)]
pub unsafe fn rdmsr(port: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") port,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Halts the CPU until the next interrupt arrives.
#[inline(always)]
pub fn hlt() {
    unsafe {
        asm!("hlt", options(nostack, nomem, preserves_flags));
    }
}

/// Disables interrupts.
pub fn cli() {
    unsafe {
        asm!("cli", options(nostack, nomem, preserves_flags));
    }
}

/// Enables interrupts.
pub fn sti() {
    unsafe {
        asm!("sti", options(nostack, nomem, preserves_flags));
    }
}

/// Invalidates the TLB entry for the given virtual address.
#[inline(always)]
pub fn invlpg(addr: usize) {
    unsafe {
        asm!("invlpg [{}]", in(reg) addr, options(nostack, readonly, preserves_flags));
    }
}
