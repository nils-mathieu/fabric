//! Wraps the raw **x86_64** system call interface.

use core::arch::asm;

/// Performs a system call with no arguments.
#[inline(always)]
pub fn syscall0(no: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            clobber_abi("C"),
        );
    }

    ret
}

/// Performs a system call with one argument.
#[inline(always)]
pub fn syscall1(no: usize, a1: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            clobber_abi("C"),
        );
    }

    ret
}

/// Performs a system call with two arguments.
#[inline(always)]
pub fn syscall2(no: usize, a1: usize, a2: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            in("rsi") a2,
            clobber_abi("C"),
        );
    }

    ret
}

/// Performs a system call with three arguments.
#[inline(always)]
pub fn syscall3(no: usize, a1: usize, a2: usize, a3: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            clobber_abi("C"),
        );
    }

    ret
}

/// Performs a system call with four arguments.
#[inline(always)]
pub fn syscall4(no: usize, a1: usize, a2: usize, a3: usize, a4: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            clobber_abi("C"),
        );
    }

    ret
}

/// Performs a system call with five arguments.
#[inline(always)]
pub fn syscall5(no: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            in("r8") a5,
            clobber_abi("C"),
        );
    }

    ret
}

/// Performs a system call with six arguments.
#[inline(always)]
pub fn syscall6(
    no: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
) -> usize {
    let ret;

    unsafe {
        asm!(
            "syscall",
            inlateout("rax") no => ret,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            in("r8") a5,
            in("r9") a6,
            clobber_abi("C"),
        );
    }

    ret
}
