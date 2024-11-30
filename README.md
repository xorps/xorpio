# xorpio
no_std / libc-free bindings for Linux userspace IO

🚧 **This project is currently under development** 🚧

## Example Usage
```rust
#![no_std]
#![no_main]

use xorpio::epoll::Epoll;

fn run() -> Result<(), xorpio::Errno> {
    let ev = Epoll::new()?;
    let () = ev.close()?;
    let _size = xorpio::io::write(xorpio::io::STDOUT, b"success!\n")?;
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
        Err(_) => {
            let _ = xorpio::io::write(xorpio::io::STDERR, b"failed!\n");
            1
        }
    };

    xorpio::process::exit(code);
}
```
