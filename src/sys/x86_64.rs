use core::arch::asm;

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall0(n: usize) -> isize {
    let ret;
    unsafe {
        asm!("syscall", inlateout("rax") n as isize => ret,
             lateout("rcx") _, lateout("r11") _, options(nostack, preserves_flags));
    }
    ret
}

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall1(n: usize, a1: usize) -> isize {
    let ret;
    unsafe {
        asm!("syscall", inlateout("rax") n as isize => ret, in("rdi") a1,
             lateout("rcx") _, lateout("r11") _, options(nostack, preserves_flags));
    }
    ret
}

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall2(n: usize, a1: usize, a2: usize) -> isize {
    let ret;
    unsafe {
        asm!("syscall", inlateout("rax") n as isize => ret, in("rdi") a1, in("rsi") a2,
             lateout("rcx") _, lateout("r11") _, options(nostack, preserves_flags));
    }
    ret
}

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall3(n: usize, a1: usize, a2: usize, a3: usize) -> isize {
    let ret;
    unsafe {
        asm!("syscall", inlateout("rax") n as isize => ret, in("rdi") a1, in("rsi") a2,
             in("rdx") a3, lateout("rcx") _, lateout("r11") _, options(nostack, preserves_flags));
    }
    ret
}

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall4(n: usize, a1: usize, a2: usize, a3: usize, a4: usize) -> isize {
    let ret;
    unsafe {
        asm!("syscall", inlateout("rax") n as isize => ret, in("rdi") a1, in("rsi") a2,
             in("rdx") a3, in("r10") a4,
             lateout("rcx") _, lateout("r11") _, options(nostack, preserves_flags));
    }
    ret
}

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall5(n: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> isize {
    let ret;
    unsafe {
        asm!("syscall", inlateout("rax") n as isize => ret, in("rdi") a1, in("rsi") a2,
             in("rdx") a3, in("r10") a4, in("r8") a5,
             lateout("rcx") _, lateout("r11") _, options(nostack, preserves_flags));
    }
    ret
}

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall6(
    n: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
) -> isize {
    let ret;
    unsafe {
        asm!("syscall", inlateout("rax") n as isize => ret, in("rdi") a1, in("rsi") a2,
             in("rdx") a3, in("r10") a4, in("r8") a5, in("r9") a6,
             lateout("rcx") _, lateout("r11") _, options(nostack, preserves_flags));
    }
    ret
}
