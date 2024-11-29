use core::arch::asm;

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall0(n: usize) -> isize {
    let ret;
    unsafe {
        asm!("svc #0", in("x8") n, lateout("x0") ret, options(nostack, preserves_flags));
    }
    ret
}

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall1(n: usize, a1: usize) -> isize {
    let ret;
    unsafe {
        asm!("svc #0", in("x8") n, inlateout("x0") a1 as isize => ret,
             options(nostack, preserves_flags));
    }
    ret
}

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall2(n: usize, a1: usize, a2: usize) -> isize {
    let ret;
    unsafe {
        asm!("svc #0", in("x8") n, inlateout("x0") a1 as isize => ret, in("x1") a2,
             options(nostack, preserves_flags));
    }
    ret
}

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall3(n: usize, a1: usize, a2: usize, a3: usize) -> isize {
    let ret;
    unsafe {
        asm!("svc #0", in("x8") n, inlateout("x0") a1 as isize => ret, in("x1") a2,
             in("x2") a3, options(nostack, preserves_flags));
    }
    ret
}

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall4(n: usize, a1: usize, a2: usize, a3: usize, a4: usize) -> isize {
    let ret;
    unsafe {
        asm!("svc #0", in("x8") n, inlateout("x0") a1 as isize => ret, in("x1") a2,
             in("x2") a3, in("x3") a4, options(nostack, preserves_flags));
    }
    ret
}

/// # Safety
/// The arguments must be valid for syscall `n`.
#[inline(always)]
pub unsafe fn syscall5(n: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> isize {
    let ret;
    unsafe {
        asm!("svc #0", in("x8") n, inlateout("x0") a1 as isize => ret, in("x1") a2,
             in("x2") a3, in("x3") a4, in("x4") a5, options(nostack, preserves_flags));
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
        asm!("svc #0", in("x8") n, inlateout("x0") a1 as isize => ret, in("x1") a2,
             in("x2") a3, in("x3") a4, in("x4") a5, in("x5") a6,
             options(nostack, preserves_flags));
    }
    ret
}
