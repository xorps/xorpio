//! Raw Linux syscalls.
//!
//! `syscallN` issues syscall `n` with `N` register arguments and returns the
//! kernel's raw result. Syscall numbers for the current architecture are in
//! [`nr`]. Use [`check`] to turn a raw result into a [`Result`].
//!
//! Exposed so callers can reach syscalls this crate doesn't wrap.

#[cfg(target_arch = "aarch64")]
#[path = "aarch64.rs"]
mod arch;
#[cfg(target_arch = "x86_64")]
#[path = "x86_64.rs"]
mod arch;

pub use arch::*;

/// Syscall numbers for the current architecture, plus [`nr::by_name`].
#[cfg(target_arch = "aarch64")]
#[path = "nr_aarch64.rs"]
pub mod nr;
/// Syscall numbers for the current architecture, plus [`nr::by_name`].
#[cfg(target_arch = "x86_64")]
#[path = "nr_x86_64.rs"]
pub mod nr;

use crate::{Errno, Result};

/// Converts a raw syscall return value into `Ok(value)` or `Err(errno)`.
///
/// The kernel reports errors as values in `-4095..=-1`.
#[inline(always)]
pub fn check(ret: isize) -> Result<usize> {
    if ret as usize > -4096isize as usize {
        Err(Errno::from_raw(-ret as i32))
    } else {
        Ok(ret as usize)
    }
}
