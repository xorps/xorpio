#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]

use crate::syscall;

pub struct Epoll(syscall::c_int);

impl Epoll {
    pub fn new() -> Result<Self, i32> {
        let fd = syscall::epoll_create1(0)?;
        Ok(Self(fd))
    }

    pub fn close(self) -> Result<(), i32> {
        let this = core::mem::ManuallyDrop::new(self);
        Ok(syscall::close(this.0)?)
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        // ignore error
        let _ = syscall::close(self.0);
    }
}
