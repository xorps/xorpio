#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
use core::arch::asm;

#[repr(C)]
pub struct EpollEvent {
    pub events: u32,
    pub data: u64,
}

pub const EPOLL_CTL_ADD: i32 = 1;
pub const EPOLL_CTL_MOD: i32 = 2;
pub const EPOLL_CTL_DEL: i32 = 3;

pub const EPOLLIN: u32 = 0x001;
pub const EPOLLOUT: u32 = 0x004;
pub const EPOLLERR: u32 = 0x008;
pub const EPOLLHUP: u32 = 0x010;
pub const EPOLLRDHUP: u32 = 0x2000;
pub const EPOLLET: u32 = 0x80000000;
pub const EPOLLONESHOT: u32 = 0x40000000;

#[allow(non_camel_case_types)]
pub type c_int = i32;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn epoll_create1(flags: c_int) -> Result<c_int, c_int> {
    const SYS_EPOLL_CREATE1: usize = 291;

    let mut res: c_int;

    unsafe {
        asm!(
            "syscall",
            in("rax")  SYS_EPOLL_CREATE1,
            in("rdi") flags,
            lateout("rax") res,
            lateout("rcx") _,
            lateout("r11") _,
        )
    };

    if res < 0 {
        Err(-res)
    } else {
        Ok(res)
    }
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn epoll_create1(flags: c_int) -> Result<c_int, c_int> {
    const SYS_EPOLL_CREATE1: usize = 20;
    let mut fd: c_int;

    unsafe {
        asm!(
            "mov x8, {syscall_num}",
            "svc 0",
            in("x0") flags,
            syscall_num = const SYS_EPOLL_CREATE1,
            lateout("x0") fd,
            lateout("x1") _,
            lateout("x2") _,
            lateout("x3") _,
            lateout("x4") _,
            lateout("x5") _,
            lateout("x6") _,
            lateout("x7") _,
        );
    }

    if fd >= 0 {
        Ok(fd)
    } else {
        Err(fd)
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn epoll_ctl(
    epfd: c_int,
    op: c_int,
    fd: c_int,
    event: *const EpollEvent,
) -> Result<c_int, c_int> {
    const SYS_EPOLL_CTL: usize = 233;

    let mut res: c_int;

    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_EPOLL_CTL,
            in("rdi") epfd,
            in("rsi") op,
            in("rdx") fd,
            in("r10") event,
            lateout("rax") res,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }

    if res < 0 {
        Err(-res)
    } else {
        Ok(res)
    }
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn epoll_ctl(
    epfd: c_int,
    op: c_int,
    fd: c_int,
    event: *const EpollEvent,
) -> Result<c_int, c_int> {
    const SYS_EPOLL_CTL: usize = 21;

    let mut res: c_int;

    unsafe {
        asm!(
            "mov x8, {syscall_num}",
            "svc 0",
            in("x0") epfd,
            in("x1") op,
            in("x2") fd,
            in("x3") event,
            syscall_num = const SYS_EPOLL_CTL,
            lateout("x0") res,
        );
    }

    if res >= 0 {
        Ok(res)
    } else {
        Err(res)
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn epoll_wait(epfd: c_int, events: &mut [EpollEvent], timeout: c_int) -> Result<c_int, c_int> {
    const SYS_EPOLL_WAIT: usize = 232;

    let mut res: c_int;

    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_EPOLL_WAIT,
            in("rdi") epfd,
            in("rsi") events.as_mut_ptr(),
            in("rdx") events.len(),
            in("r10") timeout,
            lateout("rax") res,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }

    if res < 0 {
        Err(-res)
    } else {
        Ok(res)
    }
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn epoll_wait(epfd: c_int, events: &mut [EpollEvent], timeout: c_int) -> Result<c_int, c_int> {
    const SYS_EPOLL_WAIT: usize = 22;

    let mut res: c_int;

    unsafe {
        asm!(
            "mov x8, {syscall_num}",
            "svc 0",
            in("x0") epfd,
            in("x1") events.as_mut_ptr(),
            in("x2") events.len(),
            in("x3") timeout,
            syscall_num = const SYS_EPOLL_WAIT,
            lateout("x0") res,
        );
    }

    if res >= 0 {
        Ok(res)
    } else {
        Err(res)
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn close(fd: c_int) -> Result<(), c_int> {
    const SYS_CLOSE: usize = 3;

    let mut res: c_int;

    unsafe {
        asm!(
            "syscall",
            in("rax") SYS_CLOSE,
            in("rdi") fd,
            lateout("rax") res,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }

    if res < 0 {
        Err(-res)
    } else {
        Ok(())
    }
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn close(fd: c_int) -> Result<(), c_int> {
    const SYS_CLOSE: usize = 57;

    let mut res: c_int;

    unsafe {
        asm!(
            "mov x8, {syscall_num}",
            "svc 0",
            in("x0") fd,
            syscall_num = const SYS_CLOSE,
            lateout("x0") res,
        );
    }

    if res < 0 {
        Err(res)
    } else {
        Ok(())
    }
}
