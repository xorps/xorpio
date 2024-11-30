use syscalls::{syscall, Sysno};

/// Equivalent to `exit` syscall
#[inline]
pub fn exit(code: i32) -> ! {
    let _ = unsafe { syscall!(Sysno::exit, code) };

    #[allow(clippy::empty_loop)]
    loop {}
}
