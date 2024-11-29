//! io_uring echo server on :8080.
//!
//! Each connection has exactly one operation in flight (a recv or a send),
//! plus one accept for the listener, so with `ENTRIES > MAX_CONNS` the
//! submission queue never fills and the completion queue never overflows.
//!
//! Docker's default seccomp profile blocks io_uring; run containers with
//! `--security-opt seccomp=unconfined`.
#![no_std]
#![no_main]

use xorpio::io::{AsRawFd, Fd, RawFd, STDERR, write_all};
use xorpio::net::{SocketAddrV4, TcpListener};
use xorpio::uring::{IoUring, Sqe, op, setup};

xorpio::entry!(main);

const ENTRIES: u32 = 256;
/// Connections are stored by fd, so this also caps the highest usable fd.
const MAX_CONNS: usize = 128;
const BUF_LEN: usize = 4096;

struct Conn {
    fd: Option<Fd>,
    buf: [u8; BUF_LEN],
    /// Bytes received into `buf` and how many of them have been sent back.
    len: usize,
    sent: usize,
}

impl Conn {
    const EMPTY: Self = Self {
        fd: None,
        buf: [0; BUF_LEN],
        len: 0,
        sent: 0,
    };
}

/// `user_data`: opcode in the high 32 bits, fd in the low 32.
fn tag(op: u8, fd: RawFd) -> u64 {
    (op as u64) << 32 | fd as u32 as u64
}

fn main() -> i32 {
    match run() {
        Ok(()) => 0,
        Err(_) => {
            let _ = write_all(STDERR, b"uring: fatal error\n");
            1
        }
    }
}

fn run() -> xorpio::Result<()> {
    let listener = TcpListener::bind(SocketAddrV4::new([0, 0, 0, 0], 8080), 1024)?;
    let lfd = listener.as_raw_fd();
    // The kernel writes into these buffers while operations are in flight.
    // Declared before `ring` so that the ring is torn down first.
    let mut conns: [Conn; MAX_CONNS] = [const { Conn::EMPTY }; MAX_CONNS];
    let mut ring = IoUring::with_flags(ENTRIES, setup::SINGLE_ISSUER | setup::DEFER_TASKRUN)?;

    // SAFETY (all pushes): `listener` and `conns` outlive every operation,
    // and a connection's fd and buffer are only released after the
    // completion of its single in-flight operation.
    unsafe { ring.push(&Sqe::accept(lfd).user_data(tag(op::ACCEPT, lfd)))? };
    loop {
        match ring.submit_and_wait(1) {
            Ok(_) | Err(xorpio::Errno::EINTR) => {}
            Err(e) => return Err(e),
        }
        while let Some(cqe) = ring.pop() {
            let fd = cqe.user_data() as u32 as RawFd;
            let opcode = (cqe.user_data() >> 32) as u8;
            if opcode == op::ACCEPT {
                if let Ok(new) = cqe.result() {
                    let new = new as RawFd;
                    let stream = unsafe { Fd::from_raw(new) };
                    // Over capacity: `stream` drops here, closing the connection.
                    if let Some(conn) = conns.get_mut(new as usize) {
                        let sqe = Sqe::recv(new, &mut conn.buf).user_data(tag(op::RECV, new));
                        unsafe { ring.push(&sqe)? };
                        conn.fd = Some(stream);
                    }
                }
                unsafe { ring.push(&Sqe::accept(lfd).user_data(tag(op::ACCEPT, lfd)))? };
                continue;
            }

            let Some(conn) = conns.get_mut(fd as usize) else {
                continue;
            };
            match (opcode, cqe.result()) {
                (op::RECV, Ok(n)) if n > 0 => {
                    conn.len = n as usize;
                    conn.sent = 0;
                }
                (op::SEND, Ok(n)) if n > 0 => conn.sent += n as usize,
                _ => {
                    // EOF or error: dropping the fd closes the connection.
                    conn.fd = None;
                    continue;
                }
            }
            let sqe = match conn.buf.get(conn.sent..conn.len) {
                Some(rest) if !rest.is_empty() => Sqe::send(fd, rest).user_data(tag(op::SEND, fd)),
                _ => Sqe::recv(fd, &mut conn.buf).user_data(tag(op::RECV, fd)),
            };
            unsafe { ring.push(&sqe)? };
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    xorpio::process::exit(101)
}
