use syscalls::{syscall, Errno, Sysno};

pub const STDOUT: usize = 1;

pub const STDERR: usize = 2;

#[inline]
pub fn write(fd: usize, bytes: &[u8]) -> Result<usize, Errno> {
    unsafe { syscall!(Sysno::write, fd, bytes.as_ptr(), bytes.len()) }
}
