//! Nonblocking, close-on-exec IPv4 TCP sockets.

use core::mem::size_of;

use crate::io::{AsRawFd, Fd, RawFd};
use crate::sys::{self, check, nr};
use crate::{Errno, Result};

const AF_INET: u16 = 2;
const SOCK_STREAM: usize = 1;
const SOCK_NONBLOCK: usize = 0o4000;
const SOCK_CLOEXEC: usize = 0o2000000;
const SOL_SOCKET: usize = 1;
const SO_REUSEADDR: usize = 2;
const SO_ERROR: usize = 4;
const SO_REUSEPORT: usize = 15;
const IPPROTO_TCP: usize = 6;
const TCP_NODELAY: usize = 1;

/// An IPv4 address and port.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SocketAddrV4 {
    pub ip: [u8; 4],
    pub port: u16,
}

impl SocketAddrV4 {
    #[inline]
    pub const fn new(ip: [u8; 4], port: u16) -> Self {
        Self { ip, port }
    }
}

/// `struct sockaddr_in`.
#[repr(C)]
struct SockAddrIn {
    family: u16,
    port: [u8; 2],
    addr: [u8; 4],
    zero: [u8; 8],
}

impl SockAddrIn {
    const LEN: u32 = size_of::<Self>() as u32;

    #[inline]
    const fn new(addr: SocketAddrV4) -> Self {
        Self {
            family: AF_INET,
            port: addr.port.to_be_bytes(),
            addr: addr.ip,
            zero: [0; 8],
        }
    }

    #[inline]
    const fn get(&self) -> SocketAddrV4 {
        SocketAddrV4 {
            ip: self.addr,
            port: u16::from_be_bytes(self.port),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shutdown {
    Read = 0,
    Write = 1,
    Both = 2,
}

/// A listening TCP socket.
#[derive(Debug)]
pub struct TcpListener(Fd);

impl TcpListener {
    /// Binds with `SO_REUSEADDR` and listens. Port `0` picks a free port.
    pub fn bind(addr: SocketAddrV4, backlog: u32) -> Result<Self> {
        Self::bind_with(addr, backlog, false)
    }

    /// Like [`bind`](Self::bind), also setting `SO_REUSEPORT` so that several
    /// listeners (e.g. one per thread) can share `addr`.
    pub fn bind_reuseport(addr: SocketAddrV4, backlog: u32) -> Result<Self> {
        Self::bind_with(addr, backlog, true)
    }

    fn bind_with(addr: SocketAddrV4, backlog: u32, reuseport: bool) -> Result<Self> {
        let fd = socket()?;
        setsockopt(&fd, SOL_SOCKET, SO_REUSEADDR, 1)?;
        if reuseport {
            setsockopt(&fd, SOL_SOCKET, SO_REUSEPORT, 1)?;
        }
        let sa = SockAddrIn::new(addr);
        check(unsafe {
            sys::syscall3(
                nr::BIND,
                fd.as_raw_fd() as usize,
                &sa as *const _ as usize,
                SockAddrIn::LEN as usize,
            )
        })?;
        check(unsafe { sys::syscall2(nr::LISTEN, fd.as_raw_fd() as usize, backlog as usize) })?;
        Ok(Self(fd))
    }

    /// Accepts a connection as a nonblocking, close-on-exec stream.
    ///
    /// Returns `EAGAIN` when no connection is pending.
    #[inline]
    pub fn accept(&self) -> Result<(TcpStream, SocketAddrV4)> {
        let mut sa = SockAddrIn::new(SocketAddrV4::new([0; 4], 0));
        let mut len = SockAddrIn::LEN;
        let fd = check(unsafe {
            sys::syscall4(
                nr::ACCEPT4,
                self.0.as_raw_fd() as usize,
                &mut sa as *mut _ as usize,
                &mut len as *mut _ as usize,
                SOCK_NONBLOCK | SOCK_CLOEXEC,
            )
        })?;
        Ok((TcpStream(unsafe { Fd::from_raw(fd as RawFd) }), sa.get()))
    }

    #[inline]
    pub fn local_addr(&self) -> Result<SocketAddrV4> {
        local_addr(&self.0)
    }
}

impl AsRawFd for TcpListener {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

/// A connected TCP socket.
#[derive(Debug)]
pub struct TcpStream(Fd);

impl TcpStream {
    /// Starts a nonblocking connect.
    ///
    /// The connection is established once the socket reports writable;
    /// then check [`take_error`](Self::take_error) for the outcome.
    pub fn connect(addr: SocketAddrV4) -> Result<Self> {
        let fd = socket()?;
        let sa = SockAddrIn::new(addr);
        match check(unsafe {
            sys::syscall3(
                nr::CONNECT,
                fd.as_raw_fd() as usize,
                &sa as *const _ as usize,
                SockAddrIn::LEN as usize,
            )
        }) {
            Ok(_) | Err(Errno::EINPROGRESS) => Ok(Self(fd)),
            Err(e) => Err(e),
        }
    }

    /// Returns `Ok(0)` at end of stream and `EAGAIN` when no data is ready.
    #[inline]
    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        self.0.read(buf)
    }

    /// Returns `EAGAIN` when the send buffer is full.
    #[inline]
    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        check(unsafe { sys::syscall2(nr::SHUTDOWN, self.0.as_raw_fd() as usize, how as usize) })
            .map(drop)
    }

    #[inline]
    pub fn set_nodelay(&self, on: bool) -> Result<()> {
        setsockopt(&self.0, IPPROTO_TCP, TCP_NODELAY, on as i32)
    }

    /// Reads and clears the pending socket error (`SO_ERROR`).
    #[inline]
    pub fn take_error(&self) -> Result<Option<Errno>> {
        let mut val: i32 = 0;
        let mut len = size_of::<i32>() as u32;
        check(unsafe {
            sys::syscall5(
                nr::GETSOCKOPT,
                self.0.as_raw_fd() as usize,
                SOL_SOCKET,
                SO_ERROR,
                &mut val as *mut _ as usize,
                &mut len as *mut _ as usize,
            )
        })?;
        Ok((val != 0).then_some(Errno::from_raw(val)))
    }

    #[inline]
    pub fn local_addr(&self) -> Result<SocketAddrV4> {
        local_addr(&self.0)
    }

    /// Like dropping, but reports the error from `close(2)`.
    #[inline]
    pub fn close(self) -> Result<()> {
        self.0.close()
    }
}

impl AsRawFd for TcpStream {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

#[inline]
fn socket() -> Result<Fd> {
    let fd = check(unsafe {
        sys::syscall3(
            nr::SOCKET,
            AF_INET as usize,
            SOCK_STREAM | SOCK_NONBLOCK | SOCK_CLOEXEC,
            0,
        )
    })?;
    Ok(unsafe { Fd::from_raw(fd as RawFd) })
}

#[inline]
fn setsockopt(fd: &Fd, level: usize, name: usize, val: i32) -> Result<()> {
    check(unsafe {
        sys::syscall5(
            nr::SETSOCKOPT,
            fd.as_raw_fd() as usize,
            level,
            name,
            &val as *const _ as usize,
            size_of::<i32>(),
        )
    })
    .map(drop)
}

#[inline]
fn local_addr(fd: &Fd) -> Result<SocketAddrV4> {
    let mut sa = SockAddrIn::new(SocketAddrV4::new([0; 4], 0));
    let mut len = SockAddrIn::LEN;
    check(unsafe {
        sys::syscall3(
            nr::GETSOCKNAME,
            fd.as_raw_fd() as usize,
            &mut sa as *mut _ as usize,
            &mut len as *mut _ as usize,
        )
    })?;
    Ok(sa.get())
}
