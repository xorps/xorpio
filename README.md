# xorpio

Zero-dependency, libc-free, allocation-free Linux syscalls: epoll, io_uring and
TCP for building reactor loops such as HTTP services, plus the file, mount,
namespace and process calls a container runtime needs. Think `mio`, but Linux
only and with nothing in between you and the syscalls.

- **Targets:** Linux on `x86_64` and `aarch64` only. The crate works as a
  plain `no_std` library on `*-linux-gnu`/`*-linux-musl`, and in
  freestanding `*-unknown-none` binaries started with `xorpio::entry!`.
- **No dependencies, no libc, no allocation.** Syscalls are inline `asm!`.
- **Nonblocking by default.** Sockets and epoll fds are created
  `O_NONBLOCK | O_CLOEXEC`. Would-block is `Err(Errno::EAGAIN)`.

🚧 Under development: the API is unstable.

## Modules

| Module    | Contents |
|-----------|----------|
| `epoll`   | `Epoll` (`add` / `modify` / `delete` / `wait`), `Event`, flags (`IN`, `OUT`, `RDHUP`, `ET`, `ONESHOT`, ...) |
| `uring`   | `IoUring` (`push` / `submit` / `submit_and_wait` / `pop`), `Sqe` (`accept`, `recv`, `send`, `read`, `write`, `close`, ...), `Cqe` |
| `net`     | `TcpListener`, `TcpStream`, `SocketAddrV4`, `Shutdown` |
| `io`      | `Fd` (owned, closes on drop), `AsRawFd`, `read` / `write` / `write_all` / `close`, `STDOUT` ... |
| `fs`      | `*at` file calls with `&CStr` paths: `openat`, `openat2` (`resolve::IN_ROOT` ...), `mkdirat`, `mknodat`, `renameat`, `statx`, `read_full` ... |
| `mount`   | `mount`, `pivot_root`, and the fd-based API: `open_tree`, `fsopen` / `fsconfig` / `fsmount`, `move_mount`, `mount_setattr` |
| `ns`      | `unshare`, `setns`, `sethostname`, `CLONE_NEW*` |
| `process` | `exit`, `args` / `env` (zero-copy, from `entry!`), `clone3`, `execve`, `wait4`, pidfds, credentials, `prctl`, `prlimit`, `capset` |
| `seccomp` | Classic BPF instructions and `set_mode_filter` |
| `bpf`     | eBPF `prog_load` / `prog_attach` |
| `time`    | `sleep` |
| `sys`     | Raw `syscall0`..`syscall6` and every syscall number (`sys::nr`, with `nr::by_name`), for anything not wrapped |

## Example

```rust
#![no_std]
#![no_main]

use xorpio::epoll::{self, Epoll, Event};
use xorpio::net::{SocketAddrV4, TcpListener};

xorpio::entry!(main);

fn main() -> i32 {
    run().map_or(1, |()| 0)
}

fn run() -> xorpio::Result<()> {
    let listener = TcpListener::bind(SocketAddrV4::new([0, 0, 0, 0], 8080), 1024)?;
    let ep = Epoll::new()?;
    ep.add(&listener, epoll::IN, 0)?;

    let mut events = [Event::EMPTY; 64];
    loop {
        for _ev in ep.wait(&mut events, -1)? {
            // Level-triggered: accept one connection per wakeup.
            if let Ok((stream, _peer)) = listener.accept() {
                let _ = stream.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi");
            }
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    xorpio::process::exit(101)
}
```

Complete servers are in [`examples/`](examples):
[`echo.rs`](examples/echo.rs) and [`http.rs`](examples/http.rs) (epoll), and
[`uring.rs`](examples/uring.rs) (io_uring).

io_uring needs Linux 5.4+. Docker's default seccomp profile blocks it, so
containers need `--security-opt seccomp=unconfined` (`make run` adds this for
the `uring` example). It is also unavailable under qemu-user emulation, so
`make test` skips the io_uring tests on emulated arm64.

Syscall numbers and errno names are generated from the kernel uapi headers
(`make tables`); some wrappers need a recent kernel (`openat2` 5.6,
`mount_setattr` 5.12, `clone3` with `INTO_CGROUP` 5.7).

## Development

The toolchain is pinned in `rust-toolchain.toml`.

```sh
make check                          # fmt + clippy for both targets
make test                           # integration tests on linux/amd64 and linux/arm64 (Docker)
make run EXAMPLE=http ARCH=arm64    # build a scratch image and serve on :8080
```

Freestanding builds:

```sh
cargo build --profile release-example --example http --target x86_64-unknown-none
cargo build --profile release-example --example http --target aarch64-unknown-none
```

## Releasing

```sh
cargo release alpha --execute      # 0.1.0-alpha.1 -> 0.1.0-alpha.2
cargo release release --execute    # 0.1.0-alpha.N -> 0.1.0
```

[cargo-release](https://github.com/crate-ci/cargo-release) bumps the
version, commits, tags `v<version>` and pushes. The tag runs
`.github/workflows/release.yml`, which runs CI and then publishes to
crates.io with trusted publishing.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

