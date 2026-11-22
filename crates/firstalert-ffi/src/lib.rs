//! The C interface of the FirstAlert core (the library `firstalert_ffi`), for every platform: a core is created, given JSON
//! inputs (`firstalert::Input`) one at a time, and answers each with a JSON response
//! (`firstalert::Response`). The bytes are UTF-8 without a terminating NUL.
//!
//! ```c
//! typedef struct FirstAlertCore FirstAlertCore;
//! FirstAlertCore *firstalert_core_new(void);
//! uint8_t *firstalert_core_handle(FirstAlertCore *core, const uint8_t *input, size_t length, size_t *response_length);
//! void firstalert_buffer_free(uint8_t *buffer, size_t length);
//! void firstalert_core_free(FirstAlertCore *core);
//! ```
//!
//! A core is not thread-safe: the caller serialises its calls. A panic inside the core is caught
//! and answered with an error, so a bug never takes the process down with it.
//!
//! `jni` holds the same interface for the Java Native Interface (Android, and the JVM on
//! desktops).

mod jni;

use firstalert::Core;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// An opaque core for C.
pub struct FirstAlertCore {
    core: Core,
}

impl FirstAlertCore {
    /// Handles one JSON input. Never panics: a panic of the core becomes an error response.
    pub fn handle(&mut self, input: &[u8]) -> Vec<u8> {
        let text = match std::str::from_utf8(input) {
            Ok(text) => text,
            Err(_) => return br#"{"effects":[],"error":"the input is not UTF-8"}"#.to_vec(),
        };
        let core = &mut self.core;
        match catch_unwind(AssertUnwindSafe(|| core.handle_json(text))) {
            Ok(response) => response.into_bytes(),
            Err(panic) => {
                let message = panic.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| panic.downcast_ref::<String>().cloned()).unwrap_or_default();
                let error = serde_json_string(&format!("the core failed: {message}"));
                format!(r#"{{"effects":[],"error":{error}}}"#).into_bytes()
            }
        }
    }
}

/// `text` as a JSON string literal.
fn serde_json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Creates a core. Free it with `firstalert_core_free`.
#[unsafe(no_mangle)]
pub extern "C" fn firstalert_core_new() -> *mut FirstAlertCore {
    Box::into_raw(Box::new(FirstAlertCore { core: Core::new() }))
}

/// Frees a core. A null pointer is ignored.
///
/// # Safety
/// `core` is null or a pointer from `firstalert_core_new` not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn firstalert_core_free(core: *mut FirstAlertCore) {
    if !core.is_null() {
        // SAFETY: the caller passes a pointer from `firstalert_core_new`, once.
        drop(unsafe { Box::from_raw(core) });
    }
}

/// Handles `length` bytes of JSON at `input` and returns the response, whose length is written
/// to `response_length`. Free the response with `firstalert_buffer_free`. Returns null when
/// `core` or `response_length` is null.
///
/// # Safety
/// `core` is a live core; `input` points to `length` readable bytes (or is null with `length`
/// 0); `response_length` points to a writable `size_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn firstalert_core_handle(core: *mut FirstAlertCore, input: *const u8, length: usize, response_length: *mut usize) -> *mut u8 {
    if core.is_null() || response_length.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: checked for null; the caller guarantees the rest.
    let core = unsafe { &mut *core };
    let input: &[u8] = if input.is_null() || length == 0 { &[] } else { unsafe { std::slice::from_raw_parts(input, length) } };
    let response = core.handle(input).into_boxed_slice();
    // SAFETY: checked for null.
    unsafe { *response_length = response.len() };
    Box::into_raw(response) as *mut u8
}

/// Frees a response of `firstalert_core_handle`, with the length it gave.
///
/// # Safety
/// `buffer` is null or a response of `firstalert_core_handle` not yet freed, of `length` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn firstalert_buffer_free(buffer: *mut u8, length: usize) {
    if !buffer.is_null() {
        // SAFETY: the buffer was a boxed slice of this length.
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(buffer, length)) });
    }
}
