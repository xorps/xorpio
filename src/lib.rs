//! Zero-dependency, libc-free, allocation-free Linux syscalls: epoll,
//! io_uring and TCP, plus files, mounts, namespaces, processes and seccomp.
//!
//! Supports `x86_64` and `aarch64` Linux, including freestanding
//! `*-unknown-none` binaries started with [`entry!`].
#![no_std]

#[cfg(not(all(
    any(target_os = "linux", target_os = "none"),
    any(target_arch = "x86_64", target_arch = "aarch64"),
)))]
compile_error!("xorpio supports only Linux on x86_64 and aarch64");

mod entry;
mod errno;

pub mod bpf;
pub mod epoll;
pub mod fs;
pub mod io;
pub mod mount;
pub mod net;
pub mod ns;
pub mod process;
pub mod seccomp;
pub mod sys;
pub mod time;
pub mod uring;

pub use errno::Errno;

pub type Result<T> = core::result::Result<T, Errno>;
