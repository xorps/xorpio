use syscalls::{syscall, Errno, Sysno};

#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

#[inline]
pub fn close(fd: usize) -> Result<(), Errno> {
    let res = unsafe { syscall!(Sysno::close, fd) };
    res.map(|_| ())
}

#[inline]
pub fn accept(fd: usize, flags: usize) -> Result<(usize, Option<SockAddrIn>), Errno> {
    /*
    let mut addr = SockAddrIn {
        sin_family: 0,
        sin_port: 0,
        sin_addr: 0,
        sin_zero: [0; 8],
    };
    let mut len = core::mem::size_of::<SockAddrIn>();
    let addr_ptr = &mut addr as *mut SockAddrIn as usize;
    let len_ptr = &mut len as *mut usize as usize;
    */
    let res = unsafe {
        syscall!(
            Sysno::accept4,
            fd,                         // sockfd
            core::ptr::null::<u8>(),    // addr
            core::ptr::null::<usize>(), // addrlen
            flags
        )
    };
    match res {
        Ok(fd) => Ok((fd, None)),
        Err(e) => Err(e),
    }
}

#[inline]
pub fn send(fd: usize, buf: &[u8], flags: usize) -> Result<usize, Errno> {
    let res = unsafe {
        syscall!(
            Sysno::sendto,
            fd,                      // sockfd
            buf.as_ptr(),            // buf
            buf.len(),               // len
            flags,                   // flags
            core::ptr::null::<u8>(), // dest_addr
            0                        // addrlen
        )
    };
    res
}

#[inline]
pub fn recv<const N: usize>(fd: usize, buf: &mut [u8; N]) -> Result<usize, Errno> {
    let res = unsafe { syscall!(Sysno::recvfrom, fd, buf.as_mut_ptr() as usize, N, 0, 0, 0) };
    res
}

#[repr(C)]
pub struct SigAction {
    sa_handler: usize,
    sa_flags: u64,
    sa_restorer: usize,
    sa_mask: u64,
}

#[inline]
pub fn rt_sigaction() {}
