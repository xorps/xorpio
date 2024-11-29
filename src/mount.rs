//! Mounts: the classic `mount(2)` calls and the file-descriptor based mount
//! API (Linux 5.2+, `mount_setattr` 5.12+), which attaches and configures
//! mounts without ever resolving a target path twice.

use core::ffi::CStr;
use core::mem::size_of;

use crate::Result;
use crate::io::{AsRawFd, Fd, RawFd};
use crate::sys::{self, check, nr};

pub const MS_RDONLY: u64 = 1;
pub const MS_NOSUID: u64 = 2;
pub const MS_NODEV: u64 = 4;
pub const MS_NOEXEC: u64 = 8;
pub const MS_REMOUNT: u64 = 32;
pub const MS_BIND: u64 = 4096;
pub const MS_REC: u64 = 16384;
pub const MS_UNBINDABLE: u64 = 1 << 17;
pub const MS_PRIVATE: u64 = 1 << 18;
pub const MS_SLAVE: u64 = 1 << 19;
pub const MS_SHARED: u64 = 1 << 20;

pub const MNT_DETACH: u32 = 2;

/// `MOUNT_ATTR_*` for [`MountAttr`] and [`fsmount`].
pub mod attr {
    pub const RDONLY: u64 = 0x01;
    pub const NOSUID: u64 = 0x02;
    pub const NODEV: u64 = 0x04;
    pub const NOEXEC: u64 = 0x08;
    /// The access-time field; set it in `attr_clr` to change it.
    pub const ATIME: u64 = 0x70;
    pub const RELATIME: u64 = 0x00;
    pub const NOATIME: u64 = 0x10;
    pub const STRICTATIME: u64 = 0x20;
    pub const NODIRATIME: u64 = 0x80;
    pub const IDMAP: u64 = 0x10_0000;
    pub const NOSYMFOLLOW: u64 = 0x20_0000;
}

pub const OPEN_TREE_CLONE: u32 = 1;
pub const OPEN_TREE_CLOEXEC: u32 = 0o2000000;

pub const MOVE_MOUNT_F_EMPTY_PATH: u32 = 0x04;
pub const MOVE_MOUNT_T_EMPTY_PATH: u32 = 0x40;

pub const FSOPEN_CLOEXEC: u32 = 1;
pub const FSMOUNT_CLOEXEC: u32 = 1;

/// `fsconfig(2)` commands.
pub mod fsconfig {
    pub const SET_FLAG: u32 = 0;
    pub const SET_STRING: u32 = 1;
    pub const CMD_CREATE: u32 = 6;
}

#[inline]
fn opt(s: Option<&CStr>) -> usize {
    s.map_or(0, |s| s.as_ptr() as usize)
}

#[inline]
fn fd(ret: isize) -> Result<Fd> {
    Ok(unsafe { Fd::from_raw(check(ret)? as RawFd) })
}

/// `mount(2)`.
#[inline]
pub fn mount(
    source: Option<&CStr>,
    target: &CStr,
    fstype: Option<&CStr>,
    flags: u64,
    data: Option<&CStr>,
) -> Result<()> {
    check(unsafe {
        sys::syscall5(
            nr::MOUNT,
            opt(source),
            target.as_ptr() as usize,
            opt(fstype),
            flags as usize,
            opt(data),
        )
    })
    .map(drop)
}

#[inline]
pub fn umount2(target: &CStr, flags: u32) -> Result<()> {
    check(unsafe { sys::syscall2(nr::UMOUNT2, target.as_ptr() as usize, flags as usize) }).map(drop)
}

#[inline]
pub fn pivot_root(new_root: &CStr, put_old: &CStr) -> Result<()> {
    check(unsafe {
        sys::syscall2(
            nr::PIVOT_ROOT,
            new_root.as_ptr() as usize,
            put_old.as_ptr() as usize,
        )
    })
    .map(drop)
}

/// `open_tree(2)`. With [`OPEN_TREE_CLONE`] the result is a detached copy of
/// the mount at `path` (recursive with [`AT_RECURSIVE`](crate::fs::AT_RECURSIVE)).
#[inline]
pub fn open_tree(dir: &impl AsRawFd, path: &CStr, flags: u32) -> Result<Fd> {
    fd(unsafe {
        sys::syscall3(
            nr::OPEN_TREE,
            dir.as_raw_fd() as usize,
            path.as_ptr() as usize,
            flags as usize,
        )
    })
}

/// `move_mount(2)`: attaches the mount at `from` onto `to`.
#[inline]
pub fn move_mount(
    from_dir: &impl AsRawFd,
    from: &CStr,
    to_dir: &impl AsRawFd,
    to: &CStr,
    flags: u32,
) -> Result<()> {
    check(unsafe {
        sys::syscall5(
            nr::MOVE_MOUNT,
            from_dir.as_raw_fd() as usize,
            from.as_ptr() as usize,
            to_dir.as_raw_fd() as usize,
            to.as_ptr() as usize,
            flags as usize,
        )
    })
    .map(drop)
}

/// `fsopen(2)`: starts configuring a new filesystem of type `fstype`.
#[inline]
pub fn fsopen(fstype: &CStr, flags: u32) -> Result<Fd> {
    fd(unsafe { sys::syscall2(nr::FSOPEN, fstype.as_ptr() as usize, flags as usize) })
}

/// `fsconfig(2)`. `key` and `value` are passed as given; `aux` is unused by
/// the string and flag commands and should be `0`.
#[inline]
pub fn fsconfig(
    fs: &impl AsRawFd,
    cmd: u32,
    key: Option<&CStr>,
    value: Option<&CStr>,
    aux: i32,
) -> Result<()> {
    check(unsafe {
        sys::syscall5(
            nr::FSCONFIG,
            fs.as_raw_fd() as usize,
            cmd as usize,
            opt(key),
            opt(value),
            aux as usize,
        )
    })
    .map(drop)
}

/// `fsmount(2)`: turns a created filesystem into a detached mount with the
/// [`attr`] flags `attrs`.
#[inline]
pub fn fsmount(fs: &impl AsRawFd, flags: u32, attrs: u64) -> Result<Fd> {
    fd(unsafe {
        sys::syscall3(
            nr::FSMOUNT,
            fs.as_raw_fd() as usize,
            flags as usize,
            attrs as usize,
        )
    })
}

/// `struct mount_attr` for [`mount_setattr`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MountAttr {
    /// [`attr`] flags to set.
    pub attr_set: u64,
    /// [`attr`] flags to clear.
    pub attr_clr: u64,
    /// `MS_PRIVATE`, `MS_SHARED`, `MS_SLAVE` or `MS_UNBINDABLE`, or 0.
    pub propagation: u64,
    /// User namespace fd for [`attr::IDMAP`].
    pub userns_fd: u64,
}

/// `mount_setattr(2)` (Linux 5.12+): changes properties of the mount at
/// `path`, of every mount below it too with [`AT_RECURSIVE`](crate::fs::AT_RECURSIVE).
#[inline]
pub fn mount_setattr(dir: &impl AsRawFd, path: &CStr, flags: u32, attr: &MountAttr) -> Result<()> {
    check(unsafe {
        sys::syscall5(
            nr::MOUNT_SETATTR,
            dir.as_raw_fd() as usize,
            path.as_ptr() as usize,
            flags as usize,
            attr as *const MountAttr as usize,
            size_of::<MountAttr>(),
        )
    })
    .map(drop)
}
