//! Loading and attaching eBPF programs.

use core::ffi::CStr;
use core::mem::size_of;

use crate::io::{AsRawFd, Fd, RawFd};
use crate::sys::{self, check, nr};
use crate::{Errno, Result};

/// `struct bpf_insn`: one eBPF instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Insn {
    pub code: u8,
    /// `dst_reg` in the low nibble, `src_reg` in the high nibble.
    pub regs: u8,
    pub off: i16,
    pub imm: i32,
}

impl Insn {
    #[inline]
    pub const fn new(code: u8, dst: u8, src: u8, off: i16, imm: i32) -> Self {
        Self {
            code,
            regs: (dst & 0xf) | (src << 4),
            off,
            imm,
        }
    }
}

/// eBPF opcodes.
pub mod op {
    pub const LDX_MEM_W: u8 = 0x61;
    pub const ALU32_AND_K: u8 = 0x54;
    pub const ALU32_RSH_K: u8 = 0x74;
    pub const ALU64_MOV_K: u8 = 0xb7;
    pub const ALU64_MOV_X: u8 = 0xbf;
    pub const JMP_JA: u8 = 0x05;
    pub const JMP32_JNE_K: u8 = 0x56;
    pub const JMP_EXIT: u8 = 0x95;
}

/// `BPF_PROG_TYPE_CGROUP_DEVICE`.
pub const PROG_TYPE_CGROUP_DEVICE: u32 = 15;
/// `BPF_CGROUP_DEVICE`.
pub const ATTACH_CGROUP_DEVICE: u32 = 6;
/// `BPF_F_ALLOW_MULTI`.
pub const F_ALLOW_MULTI: u32 = 1 << 1;

const PROG_LOAD: usize = 5;
const PROG_ATTACH: usize = 8;

/// Loads `insns` as a program of type `prog_type` and returns its fd.
pub fn prog_load(prog_type: u32, insns: &[Insn], license: &CStr) -> Result<Fd> {
    #[repr(C)]
    struct Attr {
        prog_type: u32,
        insn_cnt: u32,
        insns: u64,
        license: u64,
    }
    let attr = Attr {
        prog_type,
        insn_cnt: u32::try_from(insns.len()).map_err(|_| Errno::E2BIG)?,
        insns: insns.as_ptr() as u64,
        license: license.as_ptr() as u64,
    };
    let fd = check(unsafe {
        sys::syscall3(
            nr::BPF,
            PROG_LOAD,
            &attr as *const Attr as usize,
            size_of::<Attr>(),
        )
    })?;
    Ok(unsafe { Fd::from_raw(fd as RawFd) })
}

/// Attaches `prog` to `target` (e.g. a cgroup directory fd).
pub fn prog_attach(
    target: &impl AsRawFd,
    prog: &impl AsRawFd,
    attach_type: u32,
    flags: u32,
) -> Result<()> {
    #[repr(C)]
    struct Attr {
        target_fd: u32,
        attach_bpf_fd: u32,
        attach_type: u32,
        attach_flags: u32,
    }
    let attr = Attr {
        target_fd: target.as_raw_fd() as u32,
        attach_bpf_fd: prog.as_raw_fd() as u32,
        attach_type,
        attach_flags: flags,
    };
    check(unsafe {
        sys::syscall3(
            nr::BPF,
            PROG_ATTACH,
            &attr as *const Attr as usize,
            size_of::<Attr>(),
        )
    })
    .map(drop)
}
