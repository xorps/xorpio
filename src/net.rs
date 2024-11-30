use syscalls::{syscall, Errno, Sysno};

use crate::sys;

/// IPv4
const AF_INET: usize = 2;

/// For TCP
const SOCK_STREAM: usize = 1;

/// TCP
const IPPROTO_TCP: usize = 6;

pub struct TcpSocket(usize);

impl TcpSocket {
    #[inline]
    pub fn new_v4() -> Result<Self, Errno> {
        let fd = unsafe { syscall!(Sysno::socket, AF_INET, SOCK_STREAM, IPPROTO_TCP) };
        Ok(Self(fd?))
    }

    /// Same as `Drop`. Allows you to inspect error code for `close`
    #[inline]
    pub fn close(self) -> Result<(), Errno> {
        let this = core::mem::ManuallyDrop::new(self);
        sys::close(this.0)
    }

    #[inline]
    pub fn bind(self, addr: u32, port: u16) -> Result<Self, (Self, Errno)> {
        let addr = sys::SockAddrIn {
            sin_family: AF_INET as u16,
            sin_port: port.to_be(),
            sin_addr: addr.to_be(),
            sin_zero: [0; 8],
        };
        let addr = &addr as *const sys::SockAddrIn;
        let len = core::mem::size_of::<sys::SockAddrIn>();
        let res = unsafe { syscall!(Sysno::bind, self.0, addr, len) };
        match res {
            Ok(_) => Ok(self),
            Err(e) => Err((self, e)),
        }
    }

    #[inline]
    pub fn listen(self, backlog: usize) -> Result<Self, (Self, Errno)> {
        let res = unsafe { syscall!(Sysno::listen, self.0, backlog) };
        match res {
            Ok(_) => Ok(self),
            Err(e) => Err((self, e)),
        }
    }

    #[inline]
    pub fn accept(&self) -> Result<(Self, Option<sys::SockAddrIn>), Errno> {
        match sys::accept(self.0) {
            Ok((fd, addr)) => Ok((Self(fd), addr)),
            Err(err) => Err(err),
        }
    }

    #[inline]
    pub fn send(&self, buf: &[u8]) -> Result<usize, Errno> {
        sys::send(self.0, buf, 0)
    }

    #[inline]
    pub fn recv<const N: usize>(&self, buf: &mut [u8; N]) -> Result<usize, Errno> {
        sys::recv(self.0, buf)
    }
}

impl Drop for TcpSocket {
    #[inline]
    fn drop(&mut self) {
        let _ = sys::close(self.0);
    }
}
