/// Defines the process entry point `_start` for a `#![no_main]` binary.
///
/// `main` must have type `fn() -> i32`; its return value is the exit code.
/// The arguments and environment are available from
/// [`process::args`](crate::process::args) and
/// [`process::env`](crate::process::env).
/// The binary still needs its own `#[panic_handler]`.
///
/// ```ignore
/// #![no_std]
/// #![no_main]
///
/// xorpio::entry!(main);
///
/// fn main() -> i32 {
///     0
/// }
///
/// #[panic_handler]
/// fn panic(_: &core::panic::PanicInfo) -> ! {
///     xorpio::process::exit(101)
/// }
/// ```
#[macro_export]
macro_rules! entry {
    ($main:path) => {
        // The kernel enters `_start` with the stack 16-byte aligned and no
        // return address pushed, which no `extern "C" fn` expects. Clear the
        // frame pointer and re-align before calling into Rust, passing the
        // initial stack pointer, which points at argc.
        #[cfg(target_arch = "x86_64")]
        #[unsafe(no_mangle)]
        #[unsafe(naked)]
        unsafe extern "C" fn _start() -> ! {
            ::core::arch::naked_asm!(
                "xor ebp, ebp",
                "mov rdi, rsp",
                "and rsp, -16",
                "call {start}",
                "ud2",
                start = sym __xorpio_start,
            )
        }

        #[cfg(target_arch = "aarch64")]
        #[unsafe(no_mangle)]
        #[unsafe(naked)]
        unsafe extern "C" fn _start() -> ! {
            ::core::arch::naked_asm!(
                "mov x29, xzr",
                "mov x30, xzr",
                "mov x0, sp",
                "bl {start}",
                "udf #0",
                start = sym __xorpio_start,
            )
        }

        extern "C" fn __xorpio_start(sp: *mut usize) -> ! {
            $crate::process::__set_stack(sp);
            let main: fn() -> i32 = $main;
            $crate::process::exit(main())
        }
    };
}
