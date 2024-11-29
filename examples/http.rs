//! Edge-triggered epoll HTTP/1.1 server on :8080 answering every request
//! with "Hello, world!". Supports keep-alive and pipelining of requests
//! without bodies; `GET /shutdown` stops the server.
#![no_std]
#![no_main]

use xorpio::Errno;
use xorpio::epoll::{self, Epoll, Event};
use xorpio::io::{AsRawFd, STDERR, STDOUT, write_all};
use xorpio::net::{SocketAddrV4, TcpListener, TcpStream};

xorpio::entry!(main);

/// Connections are stored by fd, so this also caps the highest usable fd.
const MAX_CONNS: usize = 1024;
const LISTENER: u64 = u64::MAX;

const HELLO: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, world!";
const BYE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nBye!";
const SHUTDOWN: &[u8] = b"GET /shutdown ";

struct Conn {
    stream: TcpStream,
    /// How much of the `\r\n\r\n` request terminator has been seen.
    matched: u8,
    /// Bytes of the current request seen so far, saturating.
    len: usize,
    /// Whether the current request still matches `SHUTDOWN`.
    shutdown: bool,
}

enum Next {
    Keep,
    Close,
    Shutdown,
}

fn main() -> i32 {
    match run() {
        Ok(()) => 0,
        Err(_) => {
            let _ = write_all(STDERR, b"http: fatal error\n");
            1
        }
    }
}

fn run() -> xorpio::Result<()> {
    let listener = TcpListener::bind(SocketAddrV4::new([0, 0, 0, 0], 8080), 1024)?;
    let ep = Epoll::new()?;
    ep.add(&listener, epoll::IN | epoll::ET, LISTENER)?;
    write_all(STDOUT, b"listening on :8080\n")?;

    let mut conns: [Option<Conn>; MAX_CONNS] = [const { None }; MAX_CONNS];
    let mut events = [Event::EMPTY; 256];
    loop {
        let ready = match ep.wait(&mut events, -1) {
            Ok(ready) => ready,
            Err(Errno::EINTR) => continue,
            Err(e) => return Err(e),
        };
        for ev in ready {
            if ev.token() == LISTENER {
                accept_all(&ep, &listener, &mut conns)?;
                continue;
            }
            let Some(slot) = conns.get_mut(ev.token() as usize) else {
                continue;
            };
            let Some(conn) = slot else { continue };
            match serve(conn) {
                Next::Keep => {}
                Next::Close => *slot = None,
                Next::Shutdown => {
                    write_all(STDOUT, b"shutting down\n")?;
                    return Ok(());
                }
            }
        }
    }
}

fn accept_all(
    ep: &Epoll,
    listener: &TcpListener,
    conns: &mut [Option<Conn>],
) -> xorpio::Result<()> {
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let fd = stream.as_raw_fd() as usize;
                // Over capacity: `stream` drops here, closing the connection.
                if let Some(slot) = conns.get_mut(fd) {
                    let _ = stream.set_nodelay(true);
                    if ep
                        .add(&stream, epoll::IN | epoll::RDHUP | epoll::ET, fd as u64)
                        .is_ok()
                    {
                        *slot = Some(Conn {
                            stream,
                            matched: 0,
                            len: 0,
                            shutdown: true,
                        });
                    }
                }
            }
            Err(Errno::EAGAIN) => return Ok(()),
            Err(Errno::ECONNABORTED | Errno::EINTR) => {}
            Err(Errno::EMFILE | Errno::ENFILE) => return Ok(()),
            Err(e) => return Err(e),
        }
    }
}

/// Drains the socket and answers each complete request.
fn serve(conn: &mut Conn) -> Next {
    let mut buf = [0u8; 4096];
    loop {
        let n = match conn.stream.read(&mut buf) {
            Ok(0) => return Next::Close,
            Ok(n) => n,
            Err(Errno::EAGAIN) => return Next::Keep,
            Err(Errno::EINTR) => continue,
            Err(_) => return Next::Close,
        };
        for &b in buf.get(..n).unwrap_or_default() {
            if conn.len < SHUTDOWN.len() {
                conn.shutdown &= SHUTDOWN.get(conn.len) == Some(&b);
            }
            conn.len = conn.len.saturating_add(1);
            conn.matched = match (conn.matched, b) {
                (0 | 2, b'\r') => conn.matched + 1,
                (1 | 3, b'\n') => conn.matched + 1,
                (_, b'\r') => 1,
                _ => 0,
            };
            if conn.matched < 4 {
                continue;
            }
            let shutdown = conn.shutdown;
            if !respond(&conn.stream, if shutdown { BYE } else { HELLO }) {
                return Next::Close;
            }
            if shutdown {
                return Next::Shutdown;
            }
            conn.matched = 0;
            conn.len = 0;
            conn.shutdown = true;
        }
    }
}

/// Sends a whole response. A full send buffer would need per-connection
/// queuing; this example just drops the slow client.
fn respond(stream: &TcpStream, mut out: &[u8]) -> bool {
    while !out.is_empty() {
        match stream.write(out) {
            Ok(n) => out = out.get(n..).unwrap_or_default(),
            Err(Errno::EINTR) => {}
            Err(_) => return false,
        }
    }
    true
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    xorpio::process::exit(101)
}
