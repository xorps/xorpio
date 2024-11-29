use xorpio::Errno;
use xorpio::epoll::{self, Epoll, Event};
use xorpio::io::{self, AsRawFd};
use xorpio::net::{Shutdown, SocketAddrV4, TcpListener, TcpStream};

const LOCALHOST: SocketAddrV4 = SocketAddrV4::new([127, 0, 0, 1], 0);

fn wait_for(ep: &Epoll, token: u64) -> u32 {
    let mut events = [Event::EMPTY; 8];
    for _ in 0..100 {
        let ready = ep.wait(&mut events, 50).unwrap();
        if let Some(ev) = ready.iter().find(|ev| ev.token() == token) {
            return ev.events();
        }
    }
    panic!("timed out waiting for token {token}");
}

#[test]
fn event_layout_matches_kernel() {
    #[cfg(target_arch = "x86_64")]
    assert_eq!(size_of::<Event>(), 12);
    #[cfg(target_arch = "aarch64")]
    assert_eq!(size_of::<Event>(), 16);
}

#[test]
fn bad_fd_reports_ebadf() {
    assert_eq!(io::close(-1), Err(Errno::EBADF));
    assert_eq!(io::write(-1, b"x"), Err(Errno::EBADF));
}

#[test]
fn wait_rejects_empty_buffer() {
    let ep = Epoll::new().unwrap();
    assert_eq!(ep.wait(&mut [], 0).err(), Some(Errno::EINVAL));
}

#[test]
fn accept_on_idle_listener_would_block() {
    let listener = TcpListener::bind(LOCALHOST, 16).unwrap();
    assert_eq!(listener.accept().err(), Some(Errno::EAGAIN));
}

#[test]
fn local_addr_reports_bound_port() {
    let listener = TcpListener::bind(LOCALHOST, 16).unwrap();
    let addr = listener.local_addr().unwrap();
    assert_eq!(addr.ip, [127, 0, 0, 1]);
    assert_ne!(addr.port, 0);
}

#[test]
fn reuseport_allows_shared_bind() {
    let a = TcpListener::bind_reuseport(LOCALHOST, 16).unwrap();
    let b = TcpListener::bind_reuseport(a.local_addr().unwrap(), 16).unwrap();
    assert_eq!(a.local_addr(), b.local_addr());
}

#[test]
fn connect_refused_surfaces_via_take_error() {
    let port = TcpListener::bind(LOCALHOST, 1)
        .unwrap()
        .local_addr()
        .unwrap()
        .port;
    let ep = Epoll::new().unwrap();
    let client = match TcpStream::connect(SocketAddrV4::new([127, 0, 0, 1], port)) {
        Ok(client) => client,
        Err(e) => return assert_eq!(e, Errno::ECONNREFUSED),
    };
    ep.add(&client, epoll::OUT, 1).unwrap();
    wait_for(&ep, 1);
    assert_eq!(client.take_error(), Ok(Some(Errno::ECONNREFUSED)));
}

#[test]
fn epoll_round_trip() {
    let listener = TcpListener::bind(LOCALHOST, 16).unwrap();
    let addr = listener.local_addr().unwrap();
    let ep = Epoll::new().unwrap();
    ep.add(&listener, epoll::IN, 0).unwrap();

    let client = TcpStream::connect(addr).unwrap();
    ep.add(&client, epoll::OUT, 1).unwrap();
    assert_ne!(wait_for(&ep, 1) & epoll::OUT, 0);
    assert_eq!(client.take_error(), Ok(None));
    client.set_nodelay(true).unwrap();

    wait_for(&ep, 0);
    let (server, peer) = listener.accept().unwrap();
    assert_eq!(peer, client.local_addr().unwrap());
    ep.add(&server, epoll::IN | epoll::RDHUP, 2).unwrap();

    assert_eq!(server.read(&mut [0; 8]).err(), Some(Errno::EAGAIN));
    assert_eq!(client.write(b"ping"), Ok(4));
    assert_ne!(wait_for(&ep, 2) & epoll::IN, 0);
    let mut buf = [0; 8];
    assert_eq!(server.read(&mut buf), Ok(4));
    assert_eq!(&buf[..4], b"ping");

    ep.modify(&server, epoll::IN | epoll::RDHUP | epoll::ET, 3)
        .unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    assert_ne!(wait_for(&ep, 3) & epoll::RDHUP, 0);
    assert_eq!(server.read(&mut buf), Ok(0));

    ep.delete(&server).unwrap();
    assert_eq!(ep.delete(&server), Err(Errno::ENOENT));
    assert!(server.as_raw_fd() > 2);
    server.close().unwrap();
}
