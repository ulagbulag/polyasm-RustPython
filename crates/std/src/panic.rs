//! Panic support. A PolyASM guest builds with `panic = "abort"` and a panic traps the guest, so
//! a closure either returns or ends the guest.

use alloc_crate::boxed::Box;
use core::any::Any;

pub use core::panic::{AssertUnwindSafe, Location, PanicInfo, RefUnwindSafe, UnwindSafe};

/// Runs `f`. A panic inside `f` traps the guest, so the answer is always `Ok`.
pub fn catch_unwind<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> crate::thread::Result<R> {
    Ok(f())
}

/// Traps the guest. Under `panic = "abort"` a panic traps before any unwinding, so a payload
/// reaches here from its caller alone.
pub fn resume_unwind(payload: Box<dyn Any + Send>) -> ! {
    let _ = payload;
    panic!("resume_unwind runs under panic = \"abort\"")
}

/// Panics with `message` as payload.
pub fn panic_any<M: 'static + Any + Send>(message: M) -> ! {
    let _ = message;
    panic!("Box<dyn Any>")
}
