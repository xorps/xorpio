#![no_std]
#![no_main]

use xorpio::epoll::Epoll;

fn run() -> Result<(), xorpio::Errno> {
    let ev = Epoll::new()?;
    let () = ev.close()?;
    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    xorpio::process::exit(1)
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let code = match run() {
        Ok(()) => 0,
        Err(_) => 1,
    };

    xorpio::process::exit(code);
}
