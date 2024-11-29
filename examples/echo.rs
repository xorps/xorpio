//! Edge-triggered epoll echo server on :8080.
#![no_std]
#![no_main]

use xorpio::Errno;
use xorpio::epoll::{self, Epoll, Event};
use xorpio::io::{AsRawFd, STDERR, write_all};
use xorpio::net::{SocketAddrV4, TcpListener, TcpStream};

xorpio::entry!(main);

/// Connections are stored by fd, so this also caps the highest usable fd.
const MAX_CONNS: usize = 1024;
const LISTENER: u64 = u64::MAX;

fn main() -> i32 {
    match run() {
        Ok(()) => 0,
        Err(_) => {
            let _ = write_all(STDERR, b"echo: fatal error\n");
            1
        }
    }
}

fn run() -> xorpio::Result<()> {
    let listener = TcpListener::bind(SocketAddrV4::new([0, 0, 0, 0], 8080), 1024)?;
    let ep = Epoll::new()?;
    ep.add(&listener, epoll::IN | epoll::ET, LISTENER)?;

    let mut conns: [Option<TcpStream>; MAX_CONNS] = [const { None }; MAX_CONNS];
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
            } else if let Some(slot) = conns.get_mut(ev.token() as usize)
                && slot.as_ref().is_some_and(|s| !echo(s))
            {
                *slot = None; // Dropping closes and unregisters.
            }
        }
    }
}

fn accept_all(
    ep: &Epoll,
    listener: &TcpListener,
    conns: &mut [Option<TcpStream>],
) -> xorpio::Result<()> {
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let fd = stream.as_raw_fd() as usize;
                // Over capacity: `stream` drops here, closing the connection.
                if let Some(slot) = conns.get_mut(fd)
                    && ep
                        .add(&stream, epoll::IN | epoll::RDHUP | epoll::ET, fd as u64)
                        .is_ok()
                {
                    *slot = Some(stream);
                }
            }
            Err(Errno::EAGAIN) => return Ok(()),
            Err(Errno::ECONNABORTED | Errno::EINTR) => {}
            Err(Errno::EMFILE | Errno::ENFILE) => return Ok(()),
            Err(e) => return Err(e),
        }
    }
}

/// Drains the socket, echoing everything back. Returns `false` to close.
fn echo(stream: &TcpStream) -> bool {
    let mut buf = [0u8; 4096];
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) => return false,
            Ok(n) => n,
            Err(Errno::EAGAIN) => return true,
            Err(Errno::EINTR) => continue,
            Err(_) => return false,
        };
        let mut out = buf.get(..n).unwrap_or_default();
        while !out.is_empty() {
            match stream.write(out) {
                Ok(w) => out = out.get(w..).unwrap_or_default(),
                Err(Errno::EINTR) => {}
                // A full send buffer would need per-connection queuing;
                // this example just drops the slow client.
                Err(_) => return false,
            }
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    xorpio::process::exit(101)
}
