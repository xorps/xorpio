//! io_uring.
//!
//! Requires Linux 5.4+ (`IORING_FEAT_SINGLE_MMAP`). `IORING_SETUP_SQPOLL` is
//! not supported. Note that Docker's default seccomp profile blocks io_uring.

use core::mem::size_of;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::io::{AsRawFd, Fd, RawFd};
use crate::sys::{self, check, nr};
use crate::{Errno, Result};

/// Opcodes (`enum io_uring_op`).
pub mod op {
    pub const NOP: u8 = 0;
    pub const ACCEPT: u8 = 13;
    pub const CLOSE: u8 = 19;
    pub const READ: u8 = 22;
    pub const WRITE: u8 = 23;
    pub const SEND: u8 = 26;
    pub const RECV: u8 = 27;
}

/// `IORING_SETUP_*` flags for [`IoUring::with_flags`].
pub mod setup {
    pub const CLAMP: u32 = 1 << 4;
    pub const SUBMIT_ALL: u32 = 1 << 7;
    pub const COOP_TASKRUN: u32 = 1 << 8;
    pub const SINGLE_ISSUER: u32 = 1 << 12;
    /// Completions are only posted while waiting in
    /// [`IoUring::submit_and_wait`] with `want > 0`.
    pub const DEFER_TASKRUN: u32 = 1 << 13;
}

/// `IOSQE_*` flags for [`Sqe::flags`].
pub mod sqe {
    pub const IO_LINK: u8 = 1 << 2;
    pub const IO_HARDLINK: u8 = 1 << 3;
    pub const ASYNC: u8 = 1 << 4;
    pub const CQE_SKIP_SUCCESS: u8 = 1 << 6;
}

const FEAT_SINGLE_MMAP: u32 = 1 << 0;
const SETUP_SQPOLL: u32 = 1 << 1;
const ENTER_GETEVENTS: usize = 1 << 0;
const OFF_SQ_RING: usize = 0;
const OFF_SQES: usize = 0x1000_0000;
const CQE_F_MORE: u32 = 1 << 1;
const SOCK_CLOEXEC: u32 = 0o2000000;

/// A submission queue entry (`struct io_uring_sqe`).
///
/// Use a constructor such as [`Sqe::recv`], or fill in the fields directly
/// for operations without one.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Sqe {
    pub opcode: u8,
    pub flags: u8,
    pub ioprio: u16,
    pub fd: i32,
    pub off: u64,
    pub addr: u64,
    pub len: u32,
    pub op_flags: u32,
    pub user_data: u64,
    pub buf_index: u16,
    pub personality: u16,
    pub splice_fd_in: i32,
    pub addr3: u64,
    pub pad: u64,
}

impl Sqe {
    #[inline]
    pub fn nop() -> Self {
        Self {
            opcode: op::NOP,
            ..Self::default()
        }
    }

    /// Accepts a connection on `fd`. The result is the new close-on-exec fd.
    #[inline]
    pub fn accept(fd: RawFd) -> Self {
        Self {
            opcode: op::ACCEPT,
            fd,
            op_flags: SOCK_CLOEXEC,
            ..Self::default()
        }
    }

    #[inline]
    pub fn close(fd: RawFd) -> Self {
        Self {
            opcode: op::CLOSE,
            fd,
            ..Self::default()
        }
    }

    #[inline]
    pub fn recv(fd: RawFd, buf: &mut [u8]) -> Self {
        Self::buf(op::RECV, fd, buf.as_mut_ptr() as u64, buf.len(), 0)
    }

    #[inline]
    pub fn send(fd: RawFd, buf: &[u8]) -> Self {
        Self::buf(op::SEND, fd, buf.as_ptr() as u64, buf.len(), 0)
    }

    /// Reads at `offset`, or at the file position if `offset` is `u64::MAX`.
    #[inline]
    pub fn read(fd: RawFd, buf: &mut [u8], offset: u64) -> Self {
        Self::buf(op::READ, fd, buf.as_mut_ptr() as u64, buf.len(), offset)
    }

    /// Writes at `offset`, or at the file position if `offset` is `u64::MAX`.
    #[inline]
    pub fn write(fd: RawFd, buf: &[u8], offset: u64) -> Self {
        Self::buf(op::WRITE, fd, buf.as_ptr() as u64, buf.len(), offset)
    }

    /// Sets the value reported back in [`Cqe::user_data`].
    #[inline]
    pub const fn user_data(mut self, user_data: u64) -> Self {
        self.user_data = user_data;
        self
    }

    /// Sets `IOSQE_*` flags, see [`sqe`].
    #[inline]
    pub const fn flags(mut self, flags: u8) -> Self {
        self.flags = flags;
        self
    }

    #[inline]
    fn buf(opcode: u8, fd: RawFd, addr: u64, len: usize, off: u64) -> Self {
        let len = len.min(u32::MAX as usize) as u32;
        Self {
            opcode,
            fd,
            addr,
            len,
            off,
            ..Self::default()
        }
    }
}

/// A completion queue entry (`struct io_uring_cqe`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Cqe {
    user_data: u64,
    res: i32,
    flags: u32,
}

impl Cqe {
    /// The `user_data` of the [`Sqe`] this completes.
    #[inline]
    pub const fn user_data(&self) -> u64 {
        self.user_data
    }

    /// The operation's result, e.g. bytes transferred or an accepted fd.
    #[inline]
    pub const fn result(&self) -> Result<u32> {
        if self.res < 0 {
            Err(Errno::from_raw(-self.res))
        } else {
            Ok(self.res as u32)
        }
    }

    /// The raw `IORING_CQE_F_*` flags.
    #[inline]
    pub const fn flags(&self) -> u32 {
        self.flags
    }

    /// Whether a multishot operation will post more completions.
    #[inline]
    pub const fn has_more(&self) -> bool {
        self.flags & CQE_F_MORE != 0
    }
}

/// `struct io_uring_params`.
#[repr(C)]
#[derive(Default)]
struct Params {
    sq_entries: u32,
    cq_entries: u32,
    flags: u32,
    sq_thread_cpu: u32,
    sq_thread_idle: u32,
    features: u32,
    wq_fd: u32,
    resv: [u32; 3],
    sq_off: SqOffsets,
    cq_off: CqOffsets,
}

/// `struct io_sqring_offsets`.
#[repr(C)]
#[derive(Default)]
struct SqOffsets {
    head: u32,
    tail: u32,
    ring_mask: u32,
    ring_entries: u32,
    flags: u32,
    dropped: u32,
    array: u32,
    resv1: u32,
    user_addr: u64,
}

/// `struct io_cqring_offsets`.
#[repr(C)]
#[derive(Default)]
struct CqOffsets {
    head: u32,
    tail: u32,
    ring_mask: u32,
    ring_entries: u32,
    overflow: u32,
    cqes: u32,
    flags: u32,
    resv1: u32,
    user_addr: u64,
}

const _: () = assert!(size_of::<Params>() == 120);
const _: () = assert!(size_of::<Sqe>() == 64);
const _: () = assert!(size_of::<Cqe>() == 16);

/// An io_uring instance.
#[derive(Debug)]
pub struct IoUring {
    fd: Fd,
    ring: *mut u8,
    ring_len: usize,
    sqes: *mut Sqe,
    sqes_len: usize,
    sq_head: *const AtomicU32,
    sq_tail: *const AtomicU32,
    sq_mask: u32,
    cq_head: *const AtomicU32,
    cq_tail: *const AtomicU32,
    cq_mask: u32,
    cqes: *const Cqe,
    /// Entries pushed but not yet consumed by the kernel.
    pending: u32,
}

// The ring is owned memory; only `&mut self` methods touch it.
unsafe impl Send for IoUring {}

impl IoUring {
    /// Creates a ring with room for `entries` submissions (rounded up to a
    /// power of two) and twice as many completions.
    #[inline]
    pub fn new(entries: u32) -> Result<Self> {
        Self::with_flags(entries, 0)
    }

    /// Like [`new`](Self::new), with `IORING_SETUP_*` flags from [`setup`].
    pub fn with_flags(entries: u32, flags: u32) -> Result<Self> {
        if flags & SETUP_SQPOLL != 0 {
            return Err(Errno::EINVAL);
        }
        let mut p = Params {
            flags,
            ..Params::default()
        };
        let fd = check(unsafe {
            sys::syscall2(
                nr::IO_URING_SETUP,
                entries as usize,
                &mut p as *mut _ as usize,
            )
        })?;
        let fd = unsafe { Fd::from_raw(fd as RawFd) };
        if p.features & FEAT_SINGLE_MMAP == 0 {
            return Err(Errno::ENOSYS);
        }

        let ring_len = (p.sq_off.array as usize + p.sq_entries as usize * size_of::<u32>())
            .max(p.cq_off.cqes as usize + p.cq_entries as usize * size_of::<Cqe>());
        let ring = mmap(&fd, ring_len, OFF_SQ_RING)?;
        let sqes_len = p.sq_entries as usize * size_of::<Sqe>();
        let sqes = match mmap(&fd, sqes_len, OFF_SQES) {
            Ok(sqes) => sqes as *mut Sqe,
            Err(e) => {
                munmap(ring, ring_len);
                return Err(e);
            }
        };

        let at = |off: u32| unsafe { ring.add(off as usize) };
        unsafe {
            // Identity-map the SQ index array once: slot i always holds SQE i.
            let array = at(p.sq_off.array) as *mut u32;
            for i in 0..p.sq_entries {
                array.add(i as usize).write(i);
            }
            Ok(Self {
                fd,
                ring,
                ring_len,
                sqes,
                sqes_len,
                sq_head: at(p.sq_off.head) as *const AtomicU32,
                sq_tail: at(p.sq_off.tail) as *const AtomicU32,
                sq_mask: *(at(p.sq_off.ring_mask) as *const u32),
                cq_head: at(p.cq_off.head) as *const AtomicU32,
                cq_tail: at(p.cq_off.tail) as *const AtomicU32,
                cq_mask: *(at(p.cq_off.ring_mask) as *const u32),
                cqes: at(p.cq_off.cqes) as *const Cqe,
                pending: 0,
            })
        }
    }

    /// Queues `sqe` for the next submit. Returns `EAGAIN` if the submission
    /// queue is full.
    ///
    /// # Safety
    /// Every buffer and fd that `sqe` refers to must stay valid until its
    /// completion has been popped.
    #[inline]
    pub unsafe fn push(&mut self, sqe: &Sqe) -> Result<()> {
        unsafe {
            let head = (*self.sq_head).load(Ordering::Acquire);
            let tail = (*self.sq_tail).load(Ordering::Relaxed);
            if tail.wrapping_sub(head) > self.sq_mask {
                return Err(Errno::EAGAIN);
            }
            self.sqes.add((tail & self.sq_mask) as usize).write(*sqe);
            (*self.sq_tail).store(tail.wrapping_add(1), Ordering::Release);
        }
        self.pending += 1;
        Ok(())
    }

    /// Submits queued entries without waiting. Returns how many were taken.
    #[inline]
    pub fn submit(&mut self) -> Result<u32> {
        self.submit_and_wait(0)
    }

    /// Submits queued entries and blocks until at least `want` completions
    /// are ready. Returns how many entries were submitted.
    ///
    /// Returns `EINTR` if a signal handler interrupts the wait.
    #[inline]
    pub fn submit_and_wait(&mut self, want: u32) -> Result<u32> {
        let flags = if want > 0 { ENTER_GETEVENTS } else { 0 };
        let n = check(unsafe {
            sys::syscall6(
                nr::IO_URING_ENTER,
                self.fd.as_raw_fd() as usize,
                self.pending as usize,
                want as usize,
                flags,
                0,
                0,
            )
        })? as u32;
        self.pending -= n;
        Ok(n)
    }

    /// Takes the next completion, if any.
    #[inline]
    pub fn pop(&mut self) -> Option<Cqe> {
        unsafe {
            let head = (*self.cq_head).load(Ordering::Relaxed);
            if head == (*self.cq_tail).load(Ordering::Acquire) {
                return None;
            }
            let cqe = self.cqes.add((head & self.cq_mask) as usize).read();
            (*self.cq_head).store(head.wrapping_add(1), Ordering::Release);
            Some(cqe)
        }
    }
}

impl AsRawFd for IoUring {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl Drop for IoUring {
    fn drop(&mut self) {
        munmap(self.sqes as *mut u8, self.sqes_len);
        munmap(self.ring, self.ring_len);
        // `fd` closes after this, tearing down the ring.
    }
}

fn mmap(fd: &Fd, len: usize, offset: usize) -> Result<*mut u8> {
    const PROT_READ_WRITE: usize = 0x1 | 0x2;
    const MAP_SHARED_POPULATE: usize = 0x01 | 0x8000;
    let addr = check(unsafe {
        sys::syscall6(
            nr::MMAP,
            0,
            len,
            PROT_READ_WRITE,
            MAP_SHARED_POPULATE,
            fd.as_raw_fd() as usize,
            offset,
        )
    })?;
    Ok(addr as *mut u8)
}

fn munmap(addr: *mut u8, len: usize) {
    let _ = unsafe { sys::syscall2(nr::MUNMAP, addr as usize, len) };
}
