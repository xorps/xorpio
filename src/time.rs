//! Sleeping.

use core::time::Duration;

use crate::sys::{self, check, nr};
use crate::{Errno, Result};

/// Sleeps for `d`, resuming after signal handlers interrupt the sleep.
pub fn sleep(d: Duration) -> Result<()> {
    // struct timespec { i64 tv_sec; i64 tv_nsec; }
    let mut req: [i64; 2] = [
        i64::try_from(d.as_secs()).map_err(|_| Errno::EINVAL)?,
        i64::from(d.subsec_nanos()),
    ];
    let mut rem: [i64; 2] = [0; 2];
    loop {
        let ret = unsafe {
            sys::syscall2(
                nr::NANOSLEEP,
                req.as_ptr() as usize,
                rem.as_mut_ptr() as usize,
            )
        };
        match check(ret) {
            Ok(_) => return Ok(()),
            Err(Errno::EINTR) => req = rem,
            Err(e) => return Err(e),
        }
    }
}
