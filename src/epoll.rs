use crate::io::{AsRawFd, Fd, RawFd};
use crate::sys::{self, check, nr};
use crate::{Errno, Result};

pub const IN: u32 = 0x001;
pub const PRI: u32 = 0x002;
pub const OUT: u32 = 0x004;
pub const ERR: u32 = 0x008;
pub const HUP: u32 = 0x010;
pub const RDHUP: u32 = 0x2000;
pub const EXCLUSIVE: u32 = 1 << 28;
pub const WAKEUP: u32 = 1 << 29;
pub const ONESHOT: u32 = 1 << 30;
pub const ET: u32 = 1 << 31;

const CLOEXEC: usize = 0o2000000;
const CTL_ADD: usize = 1;
const CTL_DEL: usize = 2;
const CTL_MOD: usize = 3;

/// `struct epoll_event`. The kernel packs it on x86_64 only.
#[repr(C)]
#[cfg_attr(target_arch = "x86_64", repr(packed))]
#[derive(Clone, Copy)]
pub struct Event {
    events: u32,
    token: u64,
}

impl Event {
    pub const EMPTY: Self = Self::new(0, 0);

    #[inline]
    pub const fn new(events: u32, token: u64) -> Self {
        Self { events, token }
    }

    /// The ready flags, e.g. [`IN`] | [`HUP`].
    #[inline]
    pub const fn events(&self) -> u32 {
        self.events
    }

    /// The token passed at registration.
    #[inline]
    pub const fn token(&self) -> u64 {
        self.token
    }
}

/// An epoll instance.
#[derive(Debug)]
pub struct Epoll(Fd);

impl Epoll {
    /// Creates a close-on-exec epoll instance.
    #[inline]
    pub fn new() -> Result<Self> {
        let fd = check(unsafe { sys::syscall1(nr::EPOLL_CREATE1, CLOEXEC) })?;
        Ok(Self(unsafe { Fd::from_raw(fd as RawFd) }))
    }

    /// Registers `fd` for `events`, reported back with `token`.
    #[inline]
    pub fn add(&self, fd: &impl AsRawFd, events: u32, token: u64) -> Result<()> {
        self.ctl(CTL_ADD, fd.as_raw_fd(), &Event::new(events, token))
    }

    /// Changes the interest and token of an already registered `fd`.
    #[inline]
    pub fn modify(&self, fd: &impl AsRawFd, events: u32, token: u64) -> Result<()> {
        self.ctl(CTL_MOD, fd.as_raw_fd(), &Event::new(events, token))
    }

    /// Unregisters `fd`. Closing a descriptor also unregisters it, provided
    /// no duplicate of it remains open.
    #[inline]
    pub fn delete(&self, fd: &impl AsRawFd) -> Result<()> {
        self.ctl(CTL_DEL, fd.as_raw_fd(), &Event::EMPTY)
    }

    /// Waits up to `timeout_ms` (`-1` = forever, `0` = poll) and returns the
    /// ready events stored at the front of `events`.
    ///
    /// Returns `EINTR` if a signal handler interrupts the wait.
    #[inline]
    pub fn wait<'a>(&self, events: &'a mut [Event], timeout_ms: i32) -> Result<&'a [Event]> {
        let max = events.len().min(i32::MAX as usize);
        if max == 0 {
            return Err(Errno::EINVAL);
        }
        let n = check(unsafe {
            sys::syscall6(
                nr::EPOLL_PWAIT,
                self.0.as_raw_fd() as usize,
                events.as_mut_ptr() as usize,
                max,
                timeout_ms as usize,
                0,
                0,
            )
        })?;
        Ok(events.get(..n).unwrap_or_default())
    }

    #[inline]
    fn ctl(&self, op: usize, fd: RawFd, event: &Event) -> Result<()> {
        check(unsafe {
            sys::syscall4(
                nr::EPOLL_CTL,
                self.0.as_raw_fd() as usize,
                op,
                fd as usize,
                event as *const Event as usize,
            )
        })
        .map(drop)
    }
}

impl AsRawFd for Epoll {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}
