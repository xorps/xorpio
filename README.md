# xorpio
no_std / libc-free bindings for Linux userspace IO

🚧 **This project is currently under development** 🚧

## Example Usage
```rust
use xorpio::epoll::Epoll;

fn main() -> Result<(), i32> {
    let ev = Epoll::new()?;
    let () = ev.close()?;
    Ok(())
}
```
