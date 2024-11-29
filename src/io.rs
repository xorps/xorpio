use core::mem::ManuallyDrop;

use crate::Result;
use crate::sys::{self, check, nr};

pub type RawFd = i32;

pub const STDIN: RawFd = 0;
pub const STDOUT: RawFd = 1;
pub const STDERR: RawFd = 2;

/// Anything backed by a file descriptor.
pub trait AsRawFd {
    fn as_raw_fd(&self) -> RawFd;
}

impl AsRawFd for RawFd {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        *self
    }
}

/// An owned file descriptor, closed on drop.
#[derive(Debug)]
pub struct Fd(RawFd);

impl Fd {
    /// # Safety
    /// `fd` must be open and owned by nothing else.
    #[inline]
    pub const unsafe fn from_raw(fd: RawFd) -> Self {
        Self(fd)
    }

    #[inline]
    pub fn into_raw(self) -> RawFd {
        ManuallyDrop::new(self).0
    }

    /// Like dropping, but reports the error from `close(2)`.
    #[inline]
    pub fn close(self) -> Result<()> {
        close(self.into_raw())
    }

    #[inline]
    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        read(self.0, buf)
    }

    #[inline]
    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        write(self.0, buf)
    }
}

impl AsRawFd for Fd {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

impl Drop for Fd {
    #[inline]
    fn drop(&mut self) {
        let _ = close(self.0);
    }
}

#[inline]
pub fn read(fd: RawFd, buf: &mut [u8]) -> Result<usize> {
    check(unsafe { sys::syscall3(nr::READ, fd as usize, buf.as_mut_ptr() as usize, buf.len()) })
}

#[inline]
pub fn write(fd: RawFd, buf: &[u8]) -> Result<usize> {
    check(unsafe { sys::syscall3(nr::WRITE, fd as usize, buf.as_ptr() as usize, buf.len()) })
}

/// Writes all of `buf`, retrying on short writes and `EINTR`.
///
/// Intended for blocking descriptors such as stdout; on a nonblocking
/// descriptor this returns `EAGAIN` with part of `buf` possibly written.
pub fn write_all(fd: RawFd, mut buf: &[u8]) -> Result<()> {
    while !buf.is_empty() {
        match write(fd, buf) {
            Ok(n) => buf = buf.get(n..).unwrap_or_default(),
            Err(crate::Errno::EINTR) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// `CLOSE_RANGE_CLOEXEC`: mark the descriptors close-on-exec instead.
pub const CLOSE_RANGE_CLOEXEC: u32 = 1 << 2;

/// `close_range(2)`: closes (or with [`CLOSE_RANGE_CLOEXEC`], marks) the
/// descriptors `first..=last`.
#[inline]
pub fn close_range(first: u32, last: u32, flags: u32) -> Result<()> {
    check(unsafe {
        sys::syscall3(
            nr::CLOSE_RANGE,
            first as usize,
            last as usize,
            flags as usize,
        )
    })
    .map(drop)
}

/// `pipe2(2)`: returns the read and write ends.
#[inline]
pub fn pipe(flags: u32) -> Result<(Fd, Fd)> {
    let mut fds = [0 as RawFd; 2];
    check(unsafe { sys::syscall2(nr::PIPE2, fds.as_mut_ptr() as usize, flags as usize) })?;
    let [r, w] = fds;
    Ok(unsafe { (Fd::from_raw(r), Fd::from_raw(w)) })
}

#[inline]
pub fn close(fd: RawFd) -> Result<()> {
    check(unsafe { sys::syscall1(nr::CLOSE, fd as usize) }).map(drop)
}
