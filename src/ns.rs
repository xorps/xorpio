//! Namespaces.

use crate::Result;
use crate::io::AsRawFd;
use crate::sys::{self, check, nr};

/// `CLONE_NEW*` flags, for [`unshare`], [`setns`] and
/// [`CloneArgs::flags`](crate::process::CloneArgs::flags).
pub const NEWTIME: u64 = 0x0000_0080;
pub const NEWNS: u64 = 0x0002_0000;
pub const NEWCGROUP: u64 = 0x0200_0000;
pub const NEWUTS: u64 = 0x0400_0000;
pub const NEWIPC: u64 = 0x0800_0000;
pub const NEWUSER: u64 = 0x1000_0000;
pub const NEWPID: u64 = 0x2000_0000;
pub const NEWNET: u64 = 0x4000_0000;

#[inline]
pub fn unshare(flags: u64) -> Result<()> {
    check(unsafe { sys::syscall1(nr::UNSHARE, flags as usize) }).map(drop)
}

/// Joins the namespace behind `fd` (a `/proc/<pid>/ns/*` file or a pidfd).
/// `nstype` is one `NEW*` flag to check the namespace type, or 0.
#[inline]
pub fn setns(fd: &impl AsRawFd, nstype: u64) -> Result<()> {
    check(unsafe { sys::syscall2(nr::SETNS, fd.as_raw_fd() as usize, nstype as usize) }).map(drop)
}

#[inline]
pub fn sethostname(name: &[u8]) -> Result<()> {
    check(unsafe { sys::syscall2(nr::SETHOSTNAME, name.as_ptr() as usize, name.len()) }).map(drop)
}

#[inline]
pub fn setdomainname(name: &[u8]) -> Result<()> {
    check(unsafe { sys::syscall2(nr::SETDOMAINNAME, name.as_ptr() as usize, name.len()) }).map(drop)
}
