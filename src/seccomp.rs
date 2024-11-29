//! seccomp filters and the classic BPF they are written in.

use crate::io::{Fd, RawFd};
use crate::sys::{self, check, nr};
use crate::{Errno, Result};

/// The kernel's limit on instructions per filter (`BPF_MAXINSNS`).
pub const MAX_INSNS: usize = 4096;

/// `struct sock_filter`: one classic BPF instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SockFilter {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

impl SockFilter {
    #[inline]
    pub const fn stmt(code: u16, k: u32) -> Self {
        Self {
            code,
            jt: 0,
            jf: 0,
            k,
        }
    }

    #[inline]
    pub const fn jump(code: u16, k: u32, jt: u8, jf: u8) -> Self {
        Self { code, jt, jf, k }
    }
}

/// Classic BPF opcodes.
pub mod bpf {
    pub const LD_W_ABS: u16 = 0x20;
    pub const LDX_IMM: u16 = 0x01;
    pub const ALU_AND_K: u16 = 0x54;
    pub const JMP_JA: u16 = 0x05;
    pub const JMP_JEQ_K: u16 = 0x15;
    pub const JMP_JGT_K: u16 = 0x25;
    pub const JMP_JGE_K: u16 = 0x35;
    pub const JMP_JSET_K: u16 = 0x45;
    pub const RET_K: u16 = 0x06;
}

/// Offsets into `struct seccomp_data`.
pub mod data {
    pub const NR: u32 = 0;
    pub const ARCH: u32 = 4;
    /// Low 32 bits of argument `i` are at `args(i)`, the high bits 4 above.
    #[inline]
    pub const fn arg_lo(i: u32) -> u32 {
        16 + 8 * i
    }
    #[inline]
    pub const fn arg_hi(i: u32) -> u32 {
        arg_lo(i) + 4
    }
}

/// `SECCOMP_RET_*` actions.
pub mod ret {
    pub const KILL_PROCESS: u32 = 0x8000_0000;
    pub const KILL_THREAD: u32 = 0x0000_0000;
    pub const TRAP: u32 = 0x0003_0000;
    /// Or the errno into the low 16 bits.
    pub const ERRNO: u32 = 0x0005_0000;
    pub const USER_NOTIF: u32 = 0x7fc0_0000;
    pub const TRACE: u32 = 0x7ff0_0000;
    pub const LOG: u32 = 0x7ffc_0000;
    pub const ALLOW: u32 = 0x7fff_0000;
}

/// `SECCOMP_FILTER_FLAG_*`.
pub mod flag {
    pub const TSYNC: u32 = 1 << 0;
    pub const LOG: u32 = 1 << 1;
    pub const SPEC_ALLOW: u32 = 1 << 2;
    pub const NEW_LISTENER: u32 = 1 << 3;
    pub const WAIT_KILLABLE_RECV: u32 = 1 << 5;
}

/// The `AUDIT_ARCH_*` value of the current architecture.
#[cfg(target_arch = "x86_64")]
pub const AUDIT_ARCH: u32 = 0xC000_003E;
#[cfg(target_arch = "aarch64")]
pub const AUDIT_ARCH: u32 = 0xC000_00B7;

/// Installs `prog` with `SECCOMP_SET_MODE_FILTER`.
///
/// The caller needs `CAP_SYS_ADMIN` or no_new_privs. With
/// [`flag::NEW_LISTENER`] the result is the user-notification fd.
pub fn set_mode_filter(flags: u32, prog: &[SockFilter]) -> Result<Option<Fd>> {
    #[repr(C)]
    struct SockFprog {
        len: u16,
        filter: *const SockFilter,
    }
    let len = u16::try_from(prog.len()).map_err(|_| Errno::EINVAL)?;
    let fprog = SockFprog {
        len,
        filter: prog.as_ptr(),
    };
    const SET_MODE_FILTER: usize = 1;
    let ret = check(unsafe {
        sys::syscall3(
            nr::SECCOMP,
            SET_MODE_FILTER,
            flags as usize,
            &fprog as *const SockFprog as usize,
        )
    })?;
    Ok((flags & flag::NEW_LISTENER != 0).then(|| unsafe { Fd::from_raw(ret as RawFd) }))
}
