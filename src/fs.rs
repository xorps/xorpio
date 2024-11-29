//! Files and directories, addressed relative to directory descriptors.
//!
//! Paths are `&CStr` so they reach the kernel without copying. Pass [`CWD`]
//! as the directory to resolve relative to the working directory.

use core::ffi::CStr;
use core::mem::size_of;

use crate::io::{AsRawFd, Fd, RawFd};
use crate::sys::{self, check, nr};
use crate::{Errno, Result};

/// `AT_FDCWD`: resolve relative paths against the working directory.
pub const CWD: RawFd = -100;

/// The empty path, for [`AT_EMPTY_PATH`] and the `*_EMPTY_PATH` flags.
pub const EMPTY: &CStr = c"";

pub const O_RDONLY: u32 = 0;
pub const O_WRONLY: u32 = 0o1;
pub const O_RDWR: u32 = 0o2;
pub const O_CREAT: u32 = 0o100;
pub const O_EXCL: u32 = 0o200;
pub const O_NOCTTY: u32 = 0o400;
pub const O_TRUNC: u32 = 0o1000;
pub const O_APPEND: u32 = 0o2000;
pub const O_NONBLOCK: u32 = 0o4000;
pub const O_CLOEXEC: u32 = 0o2000000;
pub const O_PATH: u32 = 0o10000000;
#[cfg(target_arch = "x86_64")]
pub const O_DIRECTORY: u32 = 0o200000;
#[cfg(target_arch = "x86_64")]
pub const O_NOFOLLOW: u32 = 0o400000;
#[cfg(target_arch = "aarch64")]
pub const O_DIRECTORY: u32 = 0o40000;
#[cfg(target_arch = "aarch64")]
pub const O_NOFOLLOW: u32 = 0o100000;

pub const AT_SYMLINK_NOFOLLOW: u32 = 0x100;
pub const AT_REMOVEDIR: u32 = 0x200;
pub const AT_EMPTY_PATH: u32 = 0x1000;
pub const AT_RECURSIVE: u32 = 0x8000;

pub const S_IFMT: u32 = 0o170000;
pub const S_IFSOCK: u32 = 0o140000;
pub const S_IFLNK: u32 = 0o120000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFBLK: u32 = 0o060000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFCHR: u32 = 0o020000;
pub const S_IFIFO: u32 = 0o010000;

/// `RESOLVE_*` flags for [`OpenHow::resolve`].
pub mod resolve {
    pub const NO_XDEV: u64 = 0x01;
    pub const NO_MAGICLINKS: u64 = 0x02;
    pub const NO_SYMLINKS: u64 = 0x04;
    /// Fail if resolution would leave the directory.
    pub const BENEATH: u64 = 0x08;
    /// Resolve as if the directory were `/`, including `..` and symlinks.
    pub const IN_ROOT: u64 = 0x10;
}

/// `struct open_how` for [`openat2`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OpenHow {
    pub flags: u64,
    pub mode: u64,
    pub resolve: u64,
}

#[inline]
fn fd(ret: isize) -> Result<Fd> {
    Ok(unsafe { Fd::from_raw(check(ret)? as RawFd) })
}

#[inline]
fn ptr(path: &CStr) -> usize {
    path.as_ptr() as usize
}

#[inline]
pub fn openat(dir: &impl AsRawFd, path: &CStr, flags: u32, mode: u32) -> Result<Fd> {
    fd(unsafe {
        sys::syscall4(
            nr::OPENAT,
            dir.as_raw_fd() as usize,
            ptr(path),
            flags as usize,
            mode as usize,
        )
    })
}

/// `openat2(2)` (Linux 5.6+), for confining path resolution with
/// [`resolve`] flags.
#[inline]
pub fn openat2(dir: &impl AsRawFd, path: &CStr, how: &OpenHow) -> Result<Fd> {
    fd(unsafe {
        sys::syscall4(
            nr::OPENAT2,
            dir.as_raw_fd() as usize,
            ptr(path),
            how as *const OpenHow as usize,
            size_of::<OpenHow>(),
        )
    })
}

#[inline]
pub fn mkdirat(dir: &impl AsRawFd, path: &CStr, mode: u32) -> Result<()> {
    check(unsafe {
        sys::syscall3(
            nr::MKDIRAT,
            dir.as_raw_fd() as usize,
            ptr(path),
            mode as usize,
        )
    })
    .map(drop)
}

/// Creates a node; `mode` includes the file type (e.g. [`S_IFIFO`]) and `dev`
/// is the device number for [`S_IFCHR`] and [`S_IFBLK`] (see [`makedev`]).
#[inline]
pub fn mknodat(dir: &impl AsRawFd, path: &CStr, mode: u32, dev: u32) -> Result<()> {
    check(unsafe {
        sys::syscall4(
            nr::MKNODAT,
            dir.as_raw_fd() as usize,
            ptr(path),
            mode as usize,
            dev as usize,
        )
    })
    .map(drop)
}

/// Encodes a device number for [`mknodat`] the way the kernel's
/// `new_encode_dev` does. Returns `None` if `major` exceeds 12 bits or
/// `minor` 20 bits, the most the syscall can carry.
#[inline]
pub const fn makedev(major: u32, minor: u32) -> Option<u32> {
    if major > 0xfff || minor > 0xf_ffff {
        return None;
    }
    Some((minor & 0xff) | (major << 8) | ((minor & !0xff) << 12))
}

#[inline]
pub fn unlinkat(dir: &impl AsRawFd, path: &CStr, flags: u32) -> Result<()> {
    check(unsafe {
        sys::syscall3(
            nr::UNLINKAT,
            dir.as_raw_fd() as usize,
            ptr(path),
            flags as usize,
        )
    })
    .map(drop)
}

/// `renameat2(2)` with no flags: atomically replaces `new` with `old`.
#[inline]
pub fn renameat(
    old_dir: &impl AsRawFd,
    old: &CStr,
    new_dir: &impl AsRawFd,
    new: &CStr,
) -> Result<()> {
    check(unsafe {
        sys::syscall5(
            nr::RENAMEAT2,
            old_dir.as_raw_fd() as usize,
            ptr(old),
            new_dir.as_raw_fd() as usize,
            ptr(new),
            0,
        )
    })
    .map(drop)
}

#[inline]
pub fn symlinkat(target: &CStr, dir: &impl AsRawFd, path: &CStr) -> Result<()> {
    check(unsafe {
        sys::syscall3(
            nr::SYMLINKAT,
            ptr(target),
            dir.as_raw_fd() as usize,
            ptr(path),
        )
    })
    .map(drop)
}

/// Reads the target of the symlink at `path` into `buf`; returns its length.
/// The target is not NUL-terminated and is cut off if `buf` is too small.
#[inline]
pub fn readlinkat(dir: &impl AsRawFd, path: &CStr, buf: &mut [u8]) -> Result<usize> {
    check(unsafe {
        sys::syscall4(
            nr::READLINKAT,
            dir.as_raw_fd() as usize,
            ptr(path),
            buf.as_mut_ptr() as usize,
            buf.len(),
        )
    })
}

#[inline]
pub fn fchownat(dir: &impl AsRawFd, path: &CStr, uid: u32, gid: u32, flags: u32) -> Result<()> {
    check(unsafe {
        sys::syscall5(
            nr::FCHOWNAT,
            dir.as_raw_fd() as usize,
            ptr(path),
            uid as usize,
            gid as usize,
            flags as usize,
        )
    })
    .map(drop)
}

#[inline]
pub fn chdir(path: &CStr) -> Result<()> {
    check(unsafe { sys::syscall1(nr::CHDIR, ptr(path)) }).map(drop)
}

#[inline]
pub fn fchdir(dir: &impl AsRawFd) -> Result<()> {
    check(unsafe { sys::syscall1(nr::FCHDIR, dir.as_raw_fd() as usize) }).map(drop)
}

/// The fields of `struct statx` this crate exposes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stat {
    /// File type and permission bits (`S_IF*` | mode).
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
}

impl Stat {
    #[inline]
    pub const fn is_dir(&self) -> bool {
        self.mode & S_IFMT == S_IFDIR
    }
}

/// `statx(2)`. With [`AT_EMPTY_PATH`] and an empty `path`, stats `dir` itself.
pub fn statx(dir: &impl AsRawFd, path: &CStr, flags: u32) -> Result<Stat> {
    // STATX_TYPE | STATX_MODE | STATX_UID | STATX_GID | STATX_SIZE
    const MASK: usize = 0x1 | 0x2 | 0x8 | 0x10 | 0x200;
    // struct statx is 256 bytes; the fields used here sit in the first 48.
    let mut buf = [0u64; 32];
    check(unsafe {
        sys::syscall5(
            nr::STATX,
            dir.as_raw_fd() as usize,
            ptr(path),
            flags as usize,
            MASK,
            buf.as_mut_ptr() as usize,
        )
    })?;
    // stx_mask:u32 stx_blksize:u32 | stx_attributes:u64 |
    // stx_nlink:u32 stx_uid:u32 | stx_gid:u32 stx_mode:u16 pad:u16 |
    // stx_ino:u64 | stx_size:u64
    let [_, _, w2, w3, _, size, ..] = buf;
    Ok(Stat {
        uid: (w2 >> 32) as u32,
        gid: w3 as u32,
        mode: ((w3 >> 32) & 0xffff) as u32,
        size,
    })
}

/// Reads directory entries into `buf` (`getdents64(2)`); returns the bytes
/// filled, 0 at the end. Walk them with [`DirEntries`].
#[inline]
pub fn getdents(dir: &impl AsRawFd, buf: &mut [u8]) -> Result<usize> {
    check(unsafe {
        sys::syscall3(
            nr::GETDENTS64,
            dir.as_raw_fd() as usize,
            buf.as_mut_ptr() as usize,
            buf.len(),
        )
    })
}

/// A directory entry from [`getdents`].
#[derive(Clone, Copy, Debug)]
pub struct DirEntry<'a> {
    /// `DT_*`; [`DT_DIR`] for directories.
    pub kind: u8,
    pub name: &'a CStr,
}

pub const DT_DIR: u8 = 4;

/// The entries in bytes filled by [`getdents`]: `struct linux_dirent64 {
/// u64 ino; i64 off; u16 reclen; u8 type; char name[]; }`.
pub struct DirEntries<'a>(pub &'a [u8]);

impl<'a> Iterator for DirEntries<'a> {
    type Item = DirEntry<'a>;

    fn next(&mut self) -> Option<DirEntry<'a>> {
        let reclen = u16::from_ne_bytes([*self.0.get(16)?, *self.0.get(17)?]) as usize;
        let rec = self.0.get(..reclen)?;
        self.0 = self.0.get(reclen..)?;
        let kind = *rec.get(18)?;
        let name = CStr::from_bytes_until_nul(rec.get(19..)?).ok()?;
        Some(DirEntry { kind, name })
    }
}

/// Reads until `buf` is full or end of file, retrying on `EINTR`.
/// Returns the number of bytes read.
pub fn read_full(fd: &impl AsRawFd, buf: &mut [u8]) -> Result<usize> {
    let mut filled = 0;
    while let Some(rest) = buf.get_mut(filled..) {
        if rest.is_empty() {
            break;
        }
        match crate::io::read(fd.as_raw_fd(), rest) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(Errno::EINTR) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}
