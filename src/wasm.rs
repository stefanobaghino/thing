//! Hand-rolled wasm ABI — no wasm-bindgen, so the crate stays
//! dependency-free. The module is plain Rust and compiles on every
//! target, which lets the host test suite cover it; the cdylib built
//! for wasm32-unknown-unknown exports these symbols to JS.
//!
//! Protocol (see playground/index.html for the JS side):
//! 1. `ting_alloc(len)` — get a buffer, copy UTF-8 source into it.
//! 2. `ting_run(ptr, len)` — run it; returns 1 on success, 0 on error.
//! 3. `ting_result_ptr()` / `ting_result_len()` — the UTF-8 result:
//!    the program's output, with a rendered caret diagnostic appended
//!    after it when the run failed.
//! 4. `ting_dealloc(ptr, len)` — free the source buffer from step 1.
//!
//! The result buffer is owned by the module and stays valid until the
//! next `ting_run` call. wasm is single-threaded, so a thread-local is
//! effectively a global here.

use std::cell::RefCell;

thread_local! {
    static RESULT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn ting_alloc(len: usize) -> *mut u8 {
    let mut buf: Vec<u8> = Vec::with_capacity(len.max(1));
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// # Safety
/// `ptr` must come from `ting_alloc` called with the same `len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ting_dealloc(ptr: *mut u8, len: usize) {
    unsafe { drop(Vec::from_raw_parts(ptr, 0, len.max(1))) }
}

/// # Safety
/// `ptr..ptr+len` must be readable (a buffer from `ting_alloc` that
/// the caller filled with the source text).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ting_run(ptr: *const u8, len: usize) -> i32 {
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let src = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    let ok = match crate::run_source("playground", &src, &mut out, Vec::new()) {
        Ok(()) => true,
        Err(diagnostic) => {
            if !out.is_empty() && !out.ends_with(b"\n") {
                out.push(b'\n');
            }
            out.extend_from_slice(diagnostic.as_bytes());
            false
        }
    };
    RESULT.with(|r| *r.borrow_mut() = out);
    if ok { 1 } else { 0 }
}

/// Format the source instead of running it: 1 and the formatted text
/// on success, 0 and a rendered diagnostic when it doesn't parse.
///
/// # Safety
/// Same contract as `ting_run`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ting_fmt(ptr: *const u8, len: usize) -> i32 {
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let src = String::from_utf8_lossy(bytes);
    let (ok, out) = match crate::fmt::format(&src) {
        Ok(formatted) => (1, formatted.into_bytes()),
        Err(e) => (
            0,
            crate::diag::render("playground", &src, &e.message, e.span).into_bytes(),
        ),
    };
    RESULT.with(|r| *r.borrow_mut() = out);
    ok
}

#[unsafe(no_mangle)]
pub extern "C" fn ting_result_ptr() -> *const u8 {
    RESULT.with(|r| r.borrow().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn ting_result_len() -> usize {
    RESULT.with(|r| r.borrow().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the ABI exactly as the JS glue does.
    fn run_abi(src: &str) -> (i32, String) {
        let bytes = src.as_bytes();
        let ptr = ting_alloc(bytes.len());
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            let ok = ting_run(ptr, bytes.len());
            ting_dealloc(ptr, bytes.len());
            let result = std::slice::from_raw_parts(ting_result_ptr(), ting_result_len()).to_vec();
            (ok, String::from_utf8(result).unwrap())
        }
    }

    #[test]
    fn runs_a_program_through_the_abi() {
        let (ok, out) = run_abi("print(6 * 7);");
        assert_eq!((ok, out.as_str()), (1, "42\n"));
    }

    #[test]
    fn appends_diagnostic_after_partial_output() {
        let (ok, out) = run_abi("print(\"before\"); print(x);");
        assert_eq!(ok, 0);
        assert!(out.starts_with("before\n"));
        assert!(out.contains("error: undefined variable 'x'"));
    }

    #[test]
    fn formats_through_the_abi() {
        let src = "let  x=1;print( x );";
        let bytes = src.as_bytes();
        let ptr = ting_alloc(bytes.len());
        let (ok, out) = unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            let ok = ting_fmt(ptr, bytes.len());
            ting_dealloc(ptr, bytes.len());
            let result = std::slice::from_raw_parts(ting_result_ptr(), ting_result_len()).to_vec();
            (ok, String::from_utf8(result).unwrap())
        };
        assert_eq!((ok, out.as_str()), (1, "let x = 1; print(x);\n"));
    }

    #[test]
    fn result_survives_until_next_run() {
        let (_, first) = run_abi("print(1);");
        assert_eq!(first, "1\n");
        let (_, second) = run_abi("print(2);");
        assert_eq!(second, "2\n");
    }
}
