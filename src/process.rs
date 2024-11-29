//! Process lifecycle, credentials and limits.

use core::ffi::{CStr, c_char};
use core::mem::size_of;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::io::{AsRawFd, Fd, RawFd};
use crate::sys::{self, check, nr};
use crate::{Errno, Result};

pub type Pid = i32;

/// Signal numbers (identical on x86_64 and aarch64).
pub mod sig {
    pub const SIGHUP: i32 = 1;
    pub const SIGINT: i32 = 2;
    pub const SIGQUIT: i32 = 3;
    pub const SIGILL: i32 = 4;
    pub const SIGTRAP: i32 = 5;
    pub const SIGABRT: i32 = 6;
    pub const SIGBUS: i32 = 7;
    pub const SIGFPE: i32 = 8;
    pub const SIGKILL: i32 = 9;
    pub const SIGUSR1: i32 = 10;
    pub const SIGSEGV: i32 = 11;
    pub const SIGUSR2: i32 = 12;
    pub const SIGPIPE: i32 = 13;
    pub const SIGALRM: i32 = 14;
    pub const SIGTERM: i32 = 15;
    pub const SIGSTKFLT: i32 = 16;
    pub const SIGCHLD: i32 = 17;
    pub const SIGCONT: i32 = 18;
    pub const SIGSTOP: i32 = 19;
    pub const SIGTSTP: i32 = 20;
    pub const SIGTTIN: i32 = 21;
    pub const SIGTTOU: i32 = 22;
    pub const SIGURG: i32 = 23;
    pub const SIGXCPU: i32 = 24;
    pub const SIGXFSZ: i32 = 25;
    pub const SIGVTALRM: i32 = 26;
    pub const SIGPROF: i32 = 27;
    pub const SIGWINCH: i32 = 28;
    pub const SIGIO: i32 = 29;
    pub const SIGPWR: i32 = 30;
    pub const SIGSYS: i32 = 31;
}

/// Terminates every thread in the process with `code`.
#[inline]
pub fn exit(code: i32) -> ! {
    unsafe {
        sys::syscall1(nr::EXIT_GROUP, code as usize);
        // exit_group(2) never returns.
        core::hint::unreachable_unchecked()
    }
}

/// The initial stack pointer, recorded by [`entry!`](crate::entry).
static STACK: AtomicPtr<usize> = AtomicPtr::new(ptr::null_mut());

#[doc(hidden)]
#[inline]
pub fn __set_stack(sp: *mut usize) {
    STACK.store(sp, Ordering::Relaxed);
}

/// A NULL-terminated array of C strings set up by the kernel at exec, such
/// as `argv` or `envp`. The strings live for the whole process.
#[derive(Clone, Copy)]
pub struct CStrs {
    ptr: *const *const c_char,
    len: usize,
}

impl CStrs {
    const EMPTY: Self = Self {
        ptr: ptr::null(),
        len: 0,
    };

    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn get(&self, i: usize) -> Option<&'static CStr> {
        if i >= self.len {
            return None;
        }
        // The kernel lays out `len` valid NUL-terminated strings.
        Some(unsafe { CStr::from_ptr(*self.ptr.add(i)) })
    }

    /// The array itself, NULL-terminated, ready to pass to [`execve`].
    #[inline]
    pub const fn as_ptr(&self) -> *const *const c_char {
        self.ptr
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &'static CStr> + use<> {
        let all = *self;
        (0..all.len).filter_map(move |i| all.get(i))
    }
}

/// The program arguments.
///
/// Empty unless the binary was started with [`entry!`](crate::entry).
pub fn args() -> CStrs {
    let sp = STACK.load(Ordering::Relaxed);
    if sp.is_null() {
        return CStrs::EMPTY;
    }
    // At exec the stack holds argc, then argv[0..argc], then NULL.
    unsafe {
        CStrs {
            ptr: sp.add(1) as *const *const c_char,
            len: *sp,
        }
    }
}

/// The environment, as `NAME=value` strings.
///
/// Empty unless the binary was started with [`entry!`](crate::entry).
pub fn env() -> CStrs {
    let argv = args();
    if argv.ptr.is_null() {
        return CStrs::EMPTY;
    }
    // envp follows argv's terminating NULL and is itself NULL-terminated.
    unsafe {
        let ptr = argv.ptr.add(argv.len + 1);
        let mut len = 0;
        while !(*ptr.add(len)).is_null() {
            len += 1;
        }
        CStrs { ptr, len }
    }
}

#[inline]
pub fn getpid() -> Pid {
    // getpid(2) cannot fail.
    unsafe { sys::syscall0(nr::GETPID) as Pid }
}

#[inline]
pub fn getuid() -> u32 {
    unsafe { sys::syscall0(nr::GETUID) as u32 }
}

#[inline]
pub fn getgid() -> u32 {
    unsafe { sys::syscall0(nr::GETGID) as u32 }
}

#[inline]
pub fn kill(pid: Pid, sig: i32) -> Result<()> {
    check(unsafe { sys::syscall2(nr::KILL, pid as usize, sig as usize) }).map(drop)
}

/// `struct clone_args` for [`clone3`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CloneArgs {
    pub flags: u64,
    pub pidfd: u64,
    pub child_tid: u64,
    pub parent_tid: u64,
    pub exit_signal: u64,
    pub stack: u64,
    pub stack_size: u64,
    pub tls: u64,
    pub set_tid: u64,
    pub set_tid_size: u64,
    pub cgroup: u64,
}

/// `clone3` flags. Namespace flags are in [`ns`](crate::ns).
pub mod clone {
    pub const PIDFD: u64 = 0x1000;
    pub const CLEAR_SIGHAND: u64 = 0x1_0000_0000;
    pub const INTO_CGROUP: u64 = 0x2_0000_0000;
}

/// Which side of a [`clone3`] the caller is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forked {
    Child,
    Parent(Pid),
}

/// `clone3(2)` with fork semantics: `args.stack` must be 0, so the child runs
/// on a copy of the caller's stack.
///
/// With [`clone::PIDFD`], the pidfd is stored at `pidfd`, which must point
/// to an `i32` that stays writable until the call returns.
///
/// # Safety
/// The caller must be single-threaded: the child gets a copy of all memory
/// but only the calling thread, so locks held elsewhere stay held forever.
#[inline]
pub unsafe fn clone3(args: &CloneArgs) -> Result<Forked> {
    if args.stack != 0 {
        return Err(Errno::EINVAL);
    }
    let ret = check(unsafe {
        sys::syscall2(
            nr::CLONE3,
            args as *const CloneArgs as usize,
            size_of::<CloneArgs>(),
        )
    })?;
    Ok(match ret {
        0 => Forked::Child,
        pid => Forked::Parent(pid as Pid),
    })
}

/// `execve(2)`. `argv` and `envp` must be NULL-terminated.
///
/// Only returns on failure.
pub fn execve(path: &CStr, argv: &[*const c_char], envp: &[*const c_char]) -> Errno {
    if argv.last().is_none_or(|p| !p.is_null()) || envp.last().is_none_or(|p| !p.is_null()) {
        return Errno::EINVAL;
    }
    let ret = unsafe {
        sys::syscall3(
            nr::EXECVE,
            path.as_ptr() as usize,
            argv.as_ptr() as usize,
            envp.as_ptr() as usize,
        )
    };
    match check(ret) {
        Ok(_) => Errno::EINVAL,
        Err(e) => e,
    }
}

/// A `wait4(2)` status word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitStatus(i32);

impl WaitStatus {
    /// The exit code, if the process exited normally.
    #[inline]
    pub const fn exit_code(self) -> Option<i32> {
        if self.0 & 0x7f == 0 {
            Some((self.0 >> 8) & 0xff)
        } else {
            None
        }
    }

    /// The signal that killed the process, if one did.
    #[inline]
    pub const fn signal(self) -> Option<i32> {
        let sig = self.0 & 0x7f;
        if sig != 0 && sig != 0x7f {
            Some(sig)
        } else {
            None
        }
    }
}

/// `wait4(pid, &status, options, NULL)`. Returns the reaped pid and status.
pub fn wait4(pid: Pid, options: i32) -> Result<(Pid, WaitStatus)> {
    let mut status = 0i32;
    let ret = check(unsafe {
        sys::syscall4(
            nr::WAIT4,
            pid as usize,
            &mut status as *mut i32 as usize,
            options as usize,
            0,
        )
    })?;
    Ok((ret as Pid, WaitStatus(status)))
}

/// Opens a pidfd for `pid`, which pins its identity against pid reuse.
#[inline]
pub fn pidfd_open(pid: Pid) -> Result<Fd> {
    let fd = check(unsafe { sys::syscall2(nr::PIDFD_OPEN, pid as usize, 0) })?;
    Ok(unsafe { Fd::from_raw(fd as RawFd) })
}

/// Sends `sig` to the process behind `pidfd`.
#[inline]
pub fn pidfd_send_signal(pidfd: &impl AsRawFd, sig: i32) -> Result<()> {
    check(unsafe {
        sys::syscall4(
            nr::PIDFD_SEND_SIGNAL,
            pidfd.as_raw_fd() as usize,
            sig as usize,
            0,
            0,
        )
    })
    .map(drop)
}

#[inline]
pub fn setresuid(ruid: u32, euid: u32, suid: u32) -> Result<()> {
    check(unsafe { sys::syscall3(nr::SETRESUID, ruid as usize, euid as usize, suid as usize) })
        .map(drop)
}

#[inline]
pub fn setresgid(rgid: u32, egid: u32, sgid: u32) -> Result<()> {
    check(unsafe { sys::syscall3(nr::SETRESGID, rgid as usize, egid as usize, sgid as usize) })
        .map(drop)
}

#[inline]
pub fn setgroups(gids: &[u32]) -> Result<()> {
    check(unsafe { sys::syscall2(nr::SETGROUPS, gids.len(), gids.as_ptr() as usize) }).map(drop)
}

/// Sets the file mode creation mask and returns the previous one.
#[inline]
pub fn umask(mask: u32) -> u32 {
    // umask(2) cannot fail.
    unsafe { sys::syscall1(nr::UMASK, mask as usize) as u32 }
}

/// `prctl(2)` options.
pub mod pr {
    pub const SET_PDEATHSIG: usize = 1;
    pub const SET_KEEPCAPS: usize = 8;
    pub const CAPBSET_DROP: usize = 24;
    pub const SET_NO_NEW_PRIVS: usize = 38;
    pub const CAP_AMBIENT: usize = 47;
    pub const CAP_AMBIENT_RAISE: usize = 2;
    pub const CAP_AMBIENT_CLEAR_ALL: usize = 4;
}

/// Raw `prctl(2)`.
#[inline]
pub fn prctl(option: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> Result<usize> {
    check(unsafe { sys::syscall5(nr::PRCTL, option, a2, a3, a4, a5) })
}

/// `struct rlimit64`. `u64::MAX` is `RLIM_INFINITY`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rlimit {
    pub cur: u64,
    pub max: u64,
}

/// Sets resource limit `resource` (`RLIMIT_*`) of `pid` (`0` = self).
#[inline]
pub fn prlimit(pid: Pid, resource: u32, limit: &Rlimit) -> Result<()> {
    check(unsafe {
        sys::syscall4(
            nr::PRLIMIT64,
            pid as usize,
            resource as usize,
            limit as *const Rlimit as usize,
            0,
        )
    })
    .map(drop)
}

/// The capability sets of a thread, as 64-bit masks indexed by `CAP_*`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CapSets {
    pub effective: u64,
    pub permitted: u64,
    pub inheritable: u64,
}

/// `capset(2)` on the calling thread.
pub fn capset(caps: &CapSets) -> Result<()> {
    // _LINUX_CAPABILITY_VERSION_3: two 32-bit words per set.
    const VERSION_3: u32 = 0x2008_0522;
    let header: [u32; 2] = [VERSION_3, 0];
    let lo = |m: u64| m as u32;
    let hi = |m: u64| (m >> 32) as u32;
    let data: [u32; 6] = [
        lo(caps.effective),
        lo(caps.permitted),
        lo(caps.inheritable),
        hi(caps.effective),
        hi(caps.permitted),
        hi(caps.inheritable),
    ];
    check(unsafe { sys::syscall2(nr::CAPSET, header.as_ptr() as usize, data.as_ptr() as usize) })
        .map(drop)
}
