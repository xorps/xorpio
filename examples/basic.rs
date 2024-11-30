#![no_std]
#![no_main]

use xorpio::{
    io::{write, STDERR, STDOUT},
    net::TcpSocket,
    process::exit,
};

fn run() -> Result<(), xorpio::Errno> {
    let fd = TcpSocket::new_v4()?;
    let _ = write(STDOUT, b"fd created\n")?;
    let fd = fd
        .bind(u32::from_be_bytes([0, 0, 0, 0]), 8080)
        .map_err(|(_, err)| err)?;
    let _ = write(STDOUT, b"fd bind\n")?;
    let fd = fd.listen(1024).map_err(|(_, err)| err)?;
    let _ = write(STDOUT, b"server listening\n")?;
    loop {
        let (client, _) = fd.accept()?;
        let _ = write(STDOUT, b"accepted client\n")?;
        let mut buf = [0u8; 1024];
        let bytes = client.recv(&mut buf)?;
        let _ = write(STDOUT, b"received message\n")?;
        let bytes = &buf[0..bytes];
        let (close, response) = match bytes {
            b"GET /shutdown HTTP/1.1" => (
                true,
                b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\nClosing!".as_slice(),
            ),
            b"GET / HTTP/1.1" => (
                false,
                b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, world!".as_slice(),
            ),
            _ => (
                false,
                b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, world!".as_slice(),
            ),
        };
        let _ = client.send(response)?;
        let () = client.close()?;
        let _ = write(STDOUT, b"closed client\n")?;
        if close {
            let _ = write(STDOUT, b"closing server\n")?;
            return Ok(());
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1)
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let code = match run() {
        Ok(()) => 0,
        Err(err) => {
            let _ = write(STDERR, b"failed!: ");
            let _ = write(STDERR, err.name().unwrap_or("").as_bytes());
            let _ = write(STDERR, b" ");
            let _ = write(STDERR, err.description().unwrap_or("").as_bytes());
            let _ = write(STDERR, b"\n");
            1
        }
    };

    exit(code);
}
