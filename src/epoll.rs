use syscalls::{syscall, Errno, Sysno};

pub struct Epoll(usize);

impl Epoll {
    /// Creates a new epoll context
    pub fn new() -> Result<Self, Errno> {
        let fd = unsafe { syscall!(Sysno::epoll_create1, 0) };
        Ok(Self(fd?))
    }

    /// Same as `Drop`. Allows you to inspect error code for `close`
    pub fn close(self) -> Result<(), Errno> {
        let this = core::mem::ManuallyDrop::new(self);
        let res = unsafe { syscall!(Sysno::close, this.0) };
        res.map(|_| ())
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        let _ignore_err = unsafe { syscall!(Sysno::close, self.0) };
    }
}
