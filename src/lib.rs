//! r0-wasm: an R-subset interpreter core intended for WebAssembly builds.

pub mod value;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod vm;
pub mod isa;

use std::cell::RefCell;

pub use vm::R0Vm;
pub use value::R0Val;

pub const DEFAULT_FUEL: i64 = 100_000;

/// Evaluate a script in a fresh VM with `DEFAULT_FUEL`.
pub fn eval_script(src: &str) -> Result<R0Val, String> {
    R0Vm::new(DEFAULT_FUEL).eval_script(src)
}

thread_local! {
    static RESULT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn set_result(bytes: Vec<u8>) {
    RESULT.with(|r| *r.borrow_mut() = bytes);
}

/// Allocate `len` bytes in linear memory for the host to write into.
///
/// # Safety
/// The caller must release the buffer with `r0_free` using the same `len`.
#[no_mangle]
pub unsafe extern "C" fn r0_alloc(len: usize) -> *mut u8 {
    let boxed: Box<[u8]> = vec![0u8; len].into_boxed_slice();
    Box::into_raw(boxed) as *mut u8
}

/// Free a buffer previously returned by `r0_alloc`.
///
/// # Safety
/// `ptr` must come from `r0_alloc` with the same `len`, and must not be freed twice.
/// A null `ptr` with `len > 0` is a caller violation; no runtime check is made.
#[no_mangle]
pub unsafe extern "C" fn r0_free(ptr: *mut u8, len: usize) {
    // SAFETY: caller guarantees `ptr` and `len` come from `r0_alloc` and are freed once.
    // This rebuilds the exact boxed slice that `r0_alloc` leaked.
    let boxed = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)) };
    drop(boxed);
}

/// Evaluate UTF-8 R0 source at `ptr[..len]` in linear memory.
///
/// Returns 0 on success, 1 on an R0 trap, 2 on invalid UTF-8. The output
/// (Display text on success, error text on trap) is kept for `r0_result_ptr`
/// and `r0_result_len`.
///
/// # Safety
/// `ptr` must be valid for reads of `len` bytes. A null `ptr` with `len > 0`
/// is a caller violation; no runtime check is made.
#[no_mangle]
pub unsafe extern "C" fn r0_eval(ptr: *const u8, len: usize) -> i32 {
    let bytes: &[u8] = if len == 0 {
        &[]
    } else {
        // SAFETY: caller guarantees `ptr` is valid for `len` bytes, and the slice
        // does not outlive this call.
        unsafe { std::slice::from_raw_parts(ptr, len) }
    };
    let src = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            set_result(b"invalid UTF-8".to_vec());
            return 2;
        }
    };
    match eval_script(src) {
        Ok(v) => {
            set_result(v.to_string().into_bytes());
            0
        }
        Err(e) => {
            set_result(e.into_bytes());
            1
        }
    }
}

/// Pointer to the last `r0_eval` output. Valid only until the next `r0_eval`
/// on the same thread, which may replace the buffer.
#[no_mangle]
pub extern "C" fn r0_result_ptr() -> *const u8 {
    RESULT.with(|r| r.borrow().as_ptr())
}

/// Length in bytes of the last `r0_eval` output. Same validity as `r0_result_ptr`.
#[no_mangle]
pub extern "C" fn r0_result_len() -> usize {
    RESULT.with(|r| r.borrow().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_text() -> String {
        let ptr = r0_result_ptr();
        let len = r0_result_len();
        // SAFETY: `ptr`/`len` describe the thread-local result buffer, which is
        // not mutated between these two calls and the copy below.
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn eval_str(s: &str) -> i32 {
        // SAFETY: `s` is a valid &str, so its bytes are valid for `s.len()` reads.
        unsafe { r0_eval(s.as_ptr(), s.len()) }
    }

    #[test]
    fn alloc_free_round_trip() {
        for len in [0usize, 16] {
            // SAFETY: `p` is freed exactly once below with the same `len`.
            unsafe {
                let p = r0_alloc(len);
                assert!(!p.is_null());
                r0_free(p, len);
            }
        }
    }

    #[test]
    fn eval_success_returns_zero() {
        assert_eq!(eval_str("1 + 2"), 0);
        assert_eq!(result_text(), "[1] 3");
    }

    #[test]
    fn eval_trap_returns_one() {
        assert_eq!(eval_str("c(1, 2, 3, 4) + c(10, 20)"), 1);
        assert!(result_text().contains("ErrDimensionMismatch"));
    }

    #[test]
    fn eval_invalid_utf8_returns_two() {
        let bytes = [0xffu8];
        // SAFETY: `bytes` is valid for 1 read.
        let code = unsafe { r0_eval(bytes.as_ptr(), bytes.len()) };
        assert_eq!(code, 2);
        assert_eq!(result_text(), "invalid UTF-8");
    }
}
