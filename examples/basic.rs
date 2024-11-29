#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]

use xorpio::epoll::Epoll;

fn main() -> Result<(), i32> {
    let ev = Epoll::new()?;
    let () = ev.close()?;
    Ok(())
}
