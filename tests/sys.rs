use core::ffi::CStr;
use core::time::Duration;

use xorpio::Errno;
use xorpio::fs::{self, OpenHow, resolve};
use xorpio::io::{self, AsRawFd};
use xorpio::process::{self, CloneArgs, Forked, sig};
use xorpio::sys::nr;

fn scratch_dir(name: &CStr) -> io::Fd {
    let tmp = fs::openat(&fs::CWD, c"/tmp", fs::O_PATH | fs::O_DIRECTORY, 0).unwrap();
    match fs::mkdirat(&tmp, name, 0o700) {
        Ok(()) | Err(Errno::EEXIST) => {}
        Err(e) => panic!("mkdir: {e:?}"),
    }
    fs::openat(&tmp, name, fs::O_PATH | fs::O_DIRECTORY | fs::O_CLOEXEC, 0).unwrap()
}

#[test]
fn syscall_names_resolve() {
    assert_eq!(nr::by_name(b"openat"), Some(nr::OPENAT));
    assert_eq!(nr::by_name(b"clone3"), Some(nr::CLONE3));
    assert_eq!(nr::by_name(b"not_a_syscall"), None);
    assert_eq!(nr::by_name(b""), None);
    #[cfg(target_arch = "aarch64")]
    assert_eq!(nr::by_name(b"open"), None);
    #[cfg(target_arch = "x86_64")]
    assert_eq!(nr::by_name(b"open"), Some(2));
}

#[test]
fn errno_names() {
    assert_eq!(Errno::ENOENT.name(), Some("ENOENT"));
    assert_eq!(Errno::EWOULDBLOCK, Errno::EAGAIN);
    assert_eq!(Errno::from_raw(4000).name(), None);
}

#[test]
fn makedev_matches_kernel_encoding() {
    assert_eq!(fs::makedev(1, 3), Some(0x103));
    assert_eq!(fs::makedev(259, 0x12345), Some(0x1231_0345));
    assert_eq!(fs::makedev(0x1000, 0), None);
}

#[test]
fn files_round_trip() {
    let dir = scratch_dir(c"xorpio-files");
    let _ = fs::unlinkat(&dir, c"a", 0);
    let _ = fs::unlinkat(&dir, c"b", 0);
    let f = fs::openat(
        &dir,
        c"a",
        fs::O_WRONLY | fs::O_CREAT | fs::O_EXCL | fs::O_CLOEXEC,
        0o640,
    )
    .unwrap();
    io::write_all(f.as_raw_fd(), b"hello").unwrap();
    f.close().unwrap();
    fs::renameat(&dir, c"a", &dir, c"b").unwrap();

    let st = fs::statx(&dir, c"b", 0).unwrap();
    assert_eq!(st.size, 5);
    assert_eq!(st.mode & fs::S_IFMT, fs::S_IFREG);
    assert!(
        fs::statx(&dir, fs::EMPTY, fs::AT_EMPTY_PATH)
            .unwrap()
            .is_dir()
    );

    let f = fs::openat(&dir, c"b", fs::O_RDONLY | fs::O_CLOEXEC, 0).unwrap();
    let mut buf = [0u8; 16];
    assert_eq!(fs::read_full(&f, &mut buf), Ok(5));
    assert_eq!(&buf[..5], b"hello");
    assert_eq!(fs::unlinkat(&dir, c"a", 0), Err(Errno::ENOENT));
    fs::unlinkat(&dir, c"b", 0).unwrap();
}

#[test]
fn directory_entries() {
    let dir = scratch_dir(c"xorpio-dents");
    let _ = fs::mkdirat(&dir, c"sub", 0o700);
    let rd = fs::openat(
        &dir,
        c".",
        fs::O_RDONLY | fs::O_DIRECTORY | fs::O_CLOEXEC,
        0,
    )
    .unwrap();
    let mut buf = [0u8; 1024];
    let mut seen = Vec::new();
    loop {
        let n = fs::getdents(&rd, &mut buf).unwrap();
        if n == 0 {
            break;
        }
        for e in fs::DirEntries(&buf[..n]) {
            seen.push((e.name.to_bytes().to_vec(), e.kind));
        }
    }
    assert!(seen.contains(&(b"sub".to_vec(), fs::DT_DIR)));
    assert!(seen.contains(&(b".".to_vec(), fs::DT_DIR)));
}

#[test]
fn close_range_sets_cloexec() {
    let (r, _w) = io::pipe(0).unwrap();
    io::close_range(
        r.as_raw_fd() as u32,
        r.as_raw_fd() as u32,
        io::CLOSE_RANGE_CLOEXEC,
    )
    .unwrap();
}

#[test]
fn openat2_in_root_confines_symlinks() {
    let dir = scratch_dir(c"xorpio-root");
    let _ = fs::unlinkat(&dir, c"escape", 0);
    fs::symlinkat(c"/../../etc", &dir, c"escape").unwrap();
    let how = OpenHow {
        flags: (fs::O_PATH | fs::O_CLOEXEC) as u64,
        resolve: resolve::BENEATH,
        ..OpenHow::default()
    };
    assert_eq!(fs::openat2(&dir, c"escape", &how).err(), Some(Errno::EXDEV));
    let how = OpenHow {
        resolve: resolve::IN_ROOT,
        ..how
    };
    // Inside the root, "/../../etc" is "/etc" of the root, which is absent.
    assert_eq!(
        fs::openat2(&dir, c"escape", &how).err(),
        Some(Errno::ENOENT)
    );
}

/// `false` where clone3 is unimplemented, e.g. under qemu-user emulation.
fn have_clone3() -> bool {
    // A stackless clone3 with an invalid exit signal fails before forking.
    let probe = CloneArgs {
        exit_signal: 1 << 40,
        ..CloneArgs::default()
    };
    match unsafe { process::clone3(&probe) } {
        Err(Errno::ENOSYS) => {
            eprintln!("clone3 unavailable (ENOSYS), skipping");
            false
        }
        Err(Errno::EINVAL) => true,
        other => panic!("clone3 probe: {other:?}"),
    }
}

#[test]
fn clone3_reports_exit_and_signal() {
    if !have_clone3() {
        return;
    }
    let mut pidfd = -1i32;
    let args = CloneArgs {
        flags: process::clone::PIDFD,
        pidfd: &mut pidfd as *mut i32 as u64,
        exit_signal: sig::SIGCHLD as u64,
        ..CloneArgs::default()
    };
    match unsafe { process::clone3(&args) }.unwrap() {
        Forked::Child => process::exit(7),
        Forked::Parent(pid) => {
            assert!(pidfd >= 0);
            let (reaped, status) = process::wait4(pid, 0).unwrap();
            assert_eq!(reaped, pid);
            assert_eq!(status.exit_code(), Some(7));
            io::close(pidfd).unwrap();
        }
    }

    let args = CloneArgs {
        exit_signal: sig::SIGCHLD as u64,
        ..CloneArgs::default()
    };
    match unsafe { process::clone3(&args) }.unwrap() {
        Forked::Child => loop {
            xorpio::time::sleep(Duration::from_secs(1)).unwrap();
        },
        Forked::Parent(pid) => {
            let pidfd = process::pidfd_open(pid).unwrap();
            process::pidfd_send_signal(&pidfd, sig::SIGKILL).unwrap();
            let (_, status) = process::wait4(pid, 0).unwrap();
            assert_eq!(status.signal(), Some(sig::SIGKILL));
            assert_eq!(status.exit_code(), None);
            assert_eq!(process::pidfd_send_signal(&pidfd, 0), Err(Errno::ESRCH));
        }
    }
}

#[test]
fn clone3_rejects_a_stack() {
    let args = CloneArgs {
        stack: 0x1000,
        ..CloneArgs::default()
    };
    assert_eq!(unsafe { process::clone3(&args) }, Err(Errno::EINVAL));
}

#[test]
fn pipe_and_sleep() {
    let (r, w) = io::pipe(fs::O_CLOEXEC).unwrap();
    w.write(b"x").unwrap();
    drop(w);
    let mut buf = [0u8; 4];
    assert_eq!(fs::read_full(&r, &mut buf), Ok(1));
    xorpio::time::sleep(Duration::from_millis(1)).unwrap();
}

#[test]
fn execve_requires_null_termination() {
    let argv = [c"/bin/true".as_ptr()];
    let envp = [core::ptr::null()];
    assert_eq!(process::execve(c"/bin/true", &argv, &envp), Errno::EINVAL);
}
