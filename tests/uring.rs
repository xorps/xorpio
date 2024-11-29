//! Needs io_uring: under Docker, run with `--security-opt seccomp=unconfined`.

use xorpio::Errno;
use xorpio::io::AsRawFd;
use xorpio::net::{SocketAddrV4, TcpListener, TcpStream};
use xorpio::uring::{Cqe, IoUring, Sqe, setup};

/// `None` where io_uring is unimplemented, e.g. under qemu-user emulation.
fn ring(entries: u32, flags: u32) -> Option<IoUring> {
    match IoUring::with_flags(entries, flags) {
        Err(Errno::ENOSYS) => {
            eprintln!("io_uring unavailable (ENOSYS), skipping");
            None
        }
        ring => Some(ring.unwrap()),
    }
}

fn wait_one(ring: &mut IoUring) -> Cqe {
    ring.submit_and_wait(1).unwrap();
    ring.pop().expect("completion after waiting for one")
}

#[test]
fn nop_round_trip() {
    let Some(mut ring) = ring(8, 0) else { return };
    assert!(ring.pop().is_none());
    unsafe { ring.push(&Sqe::nop().user_data(42)).unwrap() };
    let cqe = wait_one(&mut ring);
    assert_eq!(cqe.user_data(), 42);
    assert_eq!(cqe.result(), Ok(0));
    assert!(ring.pop().is_none());
}

#[test]
fn full_submission_queue_reports_eagain() {
    let Some(mut ring) = ring(4, 0) else { return };
    for i in 0..4 {
        unsafe { ring.push(&Sqe::nop().user_data(i)).unwrap() };
    }
    assert_eq!(unsafe { ring.push(&Sqe::nop()) }, Err(Errno::EAGAIN));
    assert_eq!(ring.submit_and_wait(4), Ok(4));
    let mut seen = 0;
    while ring.pop().is_some() {
        seen += 1;
    }
    assert_eq!(seen, 4);
}

#[test]
fn errors_map_to_errno() {
    let Some(mut ring) = ring(8, 0) else { return };
    let mut buf = [0u8; 4];
    unsafe { ring.push(&Sqe::read(-1, &mut buf, 0).user_data(1)).unwrap() };
    assert_eq!(wait_one(&mut ring).result(), Err(Errno::EBADF));
}

#[test]
fn sqpoll_is_rejected() {
    assert_eq!(IoUring::with_flags(8, 1 << 1).err(), Some(Errno::EINVAL));
}

#[test]
fn tcp_accept_recv_send_close() {
    let Some(mut ring) = ring(8, setup::SINGLE_ISSUER | setup::DEFER_TASKRUN) else {
        return;
    };
    let listener = TcpListener::bind(SocketAddrV4::new([127, 0, 0, 1], 0), 16).unwrap();
    unsafe {
        ring.push(&Sqe::accept(listener.as_raw_fd()).user_data(1))
            .unwrap()
    };
    ring.submit().unwrap();

    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let cqe = wait_one(&mut ring);
    assert_eq!(cqe.user_data(), 1);
    let server = cqe.result().unwrap() as i32;

    let mut buf = [0u8; 16];
    unsafe {
        ring.push(&Sqe::recv(server, &mut buf).user_data(2))
            .unwrap()
    };
    ring.submit().unwrap();
    while client.write(b"ping") == Err(Errno::EAGAIN) {} // nonblocking connect
    let cqe = wait_one(&mut ring);
    assert_eq!((cqe.user_data(), cqe.result()), (2, Ok(4)));
    assert_eq!(&buf[..4], b"ping");

    unsafe { ring.push(&Sqe::send(server, b"pong").user_data(3)).unwrap() };
    let cqe = wait_one(&mut ring);
    assert_eq!((cqe.user_data(), cqe.result()), (3, Ok(4)));
    let mut got = [0u8; 4];
    loop {
        match client.read(&mut got) {
            Ok(n) => break assert_eq!(&got[..n], b"pong"),
            Err(Errno::EAGAIN) => continue,
            Err(e) => panic!("{e:?}"),
        }
    }

    unsafe { ring.push(&Sqe::close(server).user_data(4)).unwrap() };
    let cqe = wait_one(&mut ring);
    assert_eq!((cqe.user_data(), cqe.result()), (4, Ok(0)));
    assert_eq!(xorpio::io::close(server), Err(Errno::EBADF));
}
